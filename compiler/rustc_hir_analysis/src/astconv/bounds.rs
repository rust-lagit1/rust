use rustc_data_structures::fx::{FxHashMap, FxIndexSet};
use rustc_errors::struct_span_err;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::{self as ty, ToPredicate, Ty};
use rustc_span::symbol::Ident;
use rustc_span::{ErrorGuaranteed, Span};
use rustc_trait_selection::traits;
use smallvec::SmallVec;

use crate::astconv::{
    AstConv, ConvertedBinding, ConvertedBindingKind, OnlySelfBounds, PredicateFilter,
};
use crate::bounds::Bounds;
use crate::errors;

impl<'tcx> dyn AstConv<'tcx> + '_ {
    pub(crate) fn lower_where_predicates(
        &self,
        params: &[hir::GenericParam<'_>],
        hir_predicates: &[hir::WherePredicate<'_>],
        predicates: &mut FxIndexSet<(ty::Clause<'tcx>, Span)>,
    ) {
        // Collect the predicates that were written inline by the user on each
        // type parameter (e.g., `<T: Foo>`). Also add `ConstArgHasType` predicates
        // for each const parameter.
        for param in params {
            match param.kind {
                hir::GenericParamKind::Lifetime { .. } => (),
                hir::GenericParamKind::Type { .. } => {
                    let param_ty =
                        ty::fold::shift_vars(self.tcx(), self.hir_id_to_bound_ty(param.hir_id), 1);
                    let mut bounds = Bounds::default();
                    // Params are implicitly sized unless a `?Sized` bound is found
                    self.add_implicitly_sized(
                        &mut bounds,
                        param_ty,
                        &[],
                        Some((param.def_id, hir_predicates)),
                        param.span,
                    );
                    trace!(?bounds);
                    predicates.extend(bounds.clauses());
                    trace!(?predicates);
                }
                hir::GenericParamKind::Const { .. } => {
                    let ct_ty = self
                        .tcx()
                        .type_of(param.def_id.to_def_id())
                        .no_bound_vars()
                        .expect("const parameters cannot be generic");
                    let ct = ty::fold::shift_vars(
                        self.tcx(),
                        self.hir_id_to_bound_const(param.hir_id, ct_ty),
                        1,
                    );
                    predicates.insert((
                        ty::Binder::bind_with_vars(
                            ty::ClauseKind::ConstArgHasType(ct, ct_ty),
                            ty::List::empty(),
                        )
                        .to_predicate(self.tcx()),
                        param.span,
                    ));
                }
            }
        }

        // Add in the bounds that appear in the where-clause.
        for predicate in hir_predicates {
            match predicate {
                hir::WherePredicate::BoundPredicate(bound_pred) => {
                    let ty = self.ast_ty_to_ty(bound_pred.bounded_ty);
                    let bound_vars = self.tcx().late_bound_vars(bound_pred.hir_id);

                    let mut binder_predicates = FxIndexSet::default();
                    self.lower_where_predicates(
                        bound_pred.bound_generic_params,
                        bound_pred.binder_predicates,
                        &mut binder_predicates,
                    );
                    let binder_predicates = self.tcx().mk_clauses_from_iter(
                        binder_predicates.into_iter().map(|(clause, _)| clause),
                    );

                    // Keep the type around in a dummy predicate, in case of no bounds.
                    // That way, `where Ty:` is not a complete noop (see #53696) and `Ty`
                    // is still checked for WF.
                    if bound_pred.bounds.is_empty() {
                        if let ty::Param(_) = ty.kind() {
                            // This is a `where T:`, which can be in the HIR from the
                            // transformation that moves `?Sized` to `T`'s declaration.
                            // We can skip the predicate because type parameters are
                            // trivially WF, but also we *should*, to avoid exposing
                            // users who never wrote `where Type:,` themselves, to
                            // compiler/tooling bugs from not handling WF predicates.
                        } else {
                            let span = bound_pred.bounded_ty.span;
                            let predicate = ty::Binder::bind_with_vars(
                                ty::ClauseKind::WellFormed(ty.into()),
                                bound_vars,
                            );
                            predicates.insert((predicate.to_predicate(self.tcx()), span));
                        }
                    }

                    let mut bounds = Bounds::default();
                    self.add_bounds(
                        ty,
                        bound_pred.bounds.iter(),
                        &mut bounds,
                        bound_vars,
                        binder_predicates,
                        OnlySelfBounds(false),
                    );
                    predicates.extend(bounds.clauses());
                }

                hir::WherePredicate::RegionPredicate(region_pred) => {
                    let r1 = self.ast_region_to_region(&region_pred.lifetime, None);
                    predicates.extend(region_pred.bounds.iter().map(|bound| {
                        let (r2, span) = match bound {
                            hir::GenericBound::Outlives(lt) => {
                                (self.ast_region_to_region(lt, None), lt.ident.span)
                            }
                            _ => bug!(),
                        };
                        let pred = ty::ClauseKind::RegionOutlives(ty::OutlivesPredicate(r1, r2));
                        // This predicate may have escaping bound vars, e.g. if
                        // we have `for<'a: 'a> ..`. Since outlives predicates
                        // don't implicitly have a binder added for them in
                        // resolve_bound_vars, we need to explicitly shift the
                        // vars in once here.
                        let pred = ty::Binder::bind_with_vars(
                            ty::fold::shift_vars(self.tcx(), pred, 1),
                            ty::List::empty(),
                        )
                        .to_predicate(self.tcx());
                        (pred, span)
                    }))
                }

                hir::WherePredicate::EqPredicate(..) => {
                    // FIXME(#20041)
                }
            }
        }
    }

    /// Sets `implicitly_sized` to true on `Bounds` if necessary
    pub(crate) fn add_implicitly_sized<'hir>(
        &self,
        bounds: &mut Bounds<'tcx>,
        self_ty: Ty<'tcx>,
        ast_bounds: &'hir [hir::GenericBound<'hir>],
        self_ty_where_predicates: Option<(LocalDefId, &'hir [hir::WherePredicate<'hir>])>,
        span: Span,
    ) {
        let tcx = self.tcx();

        // Try to find an unbound in bounds.
        let mut unbounds: SmallVec<[_; 1]> = SmallVec::new();
        let mut search_bounds = |ast_bounds: &'hir [hir::GenericBound<'hir>]| {
            for ab in ast_bounds {
                if let hir::GenericBound::Trait(ptr, hir::TraitBoundModifier::Maybe) = ab {
                    unbounds.push(ptr);
                }
            }
        };
        search_bounds(ast_bounds);
        if let Some((self_ty, where_clause)) = self_ty_where_predicates {
            for clause in where_clause {
                if let hir::WherePredicate::BoundPredicate(pred) = clause {
                    if pred.is_param_bound(self_ty.to_def_id()) {
                        search_bounds(pred.bounds);
                    }
                }
            }
        }

        if unbounds.len() > 1 {
            tcx.sess.emit_err(errors::MultipleRelaxedDefaultBounds {
                spans: unbounds.iter().map(|ptr| ptr.span).collect(),
            });
        }

        let sized_def_id = tcx.lang_items().sized_trait();

        let mut seen_sized_unbound = false;
        for unbound in unbounds {
            if let Some(sized_def_id) = sized_def_id {
                if unbound.trait_ref.path.res == Res::Def(DefKind::Trait, sized_def_id) {
                    seen_sized_unbound = true;
                    continue;
                }
            }
            // There was a `?Trait` bound, but it was not `?Sized`; warn.
            tcx.sess.span_warn(
                unbound.span,
                "relaxing a default bound only does something for `?Sized`; \
                all other traits are not bound by default",
            );
        }

        // If the above loop finished there was no `?Sized` bound; add implicitly sized if `Sized` is available.
        if sized_def_id.is_none() {
            // No lang item for `Sized`, so we can't add it as a bound.
            return;
        }
        if seen_sized_unbound {
            // There was in fact a `?Sized` bound, return without doing anything
        } else {
            // There was no `?Sized` bound; add implicitly sized if `Sized` is available.
            bounds.push_sized(tcx, self_ty, span);
        }
    }

    /// This helper takes a *converted* parameter type (`param_ty`)
    /// and an *unconverted* list of bounds:
    ///
    /// ```text
    /// fn foo<T: Debug>
    ///        ^  ^^^^^ `ast_bounds` parameter, in HIR form
    ///        |
    ///        `param_ty`, in ty form
    /// ```
    ///
    /// It adds these `ast_bounds` into the `bounds` structure.
    ///
    /// **A note on binders:** there is an implied binder around
    /// `param_ty` and `ast_bounds`. See `instantiate_poly_trait_ref`
    /// for more details.
    #[instrument(level = "debug", skip(self, ast_bounds, bounds))]
    pub(crate) fn add_bounds<'hir, I: Iterator<Item = &'hir hir::GenericBound<'hir>>>(
        &self,
        param_ty: Ty<'tcx>,
        ast_bounds: I,
        bounds: &mut Bounds<'tcx>,
        bound_vars: &'tcx ty::List<ty::BoundVariableKind>,
        binder_predicates: &'tcx ty::List<ty::Clause<'tcx>>,
        only_self_bounds: OnlySelfBounds,
    ) {
        for ast_bound in ast_bounds {
            match ast_bound {
                hir::GenericBound::Trait(poly_trait_ref, modifier) => {
                    let (constness, polarity) = match modifier {
                        hir::TraitBoundModifier::MaybeConst => {
                            (ty::BoundConstness::ConstIfConst, ty::ImplPolarity::Positive)
                        }
                        hir::TraitBoundModifier::None => {
                            (ty::BoundConstness::NotConst, ty::ImplPolarity::Positive)
                        }
                        hir::TraitBoundModifier::Negative => {
                            (ty::BoundConstness::NotConst, ty::ImplPolarity::Negative)
                        }
                        hir::TraitBoundModifier::Maybe => continue,
                    };

                    let mut additional_binder_predicates = FxIndexSet::default();
                    self.lower_where_predicates(
                        poly_trait_ref.bound_generic_params,
                        poly_trait_ref.binder_predicates,
                        &mut additional_binder_predicates,
                    );
                    let binder_predicates =
                        self.tcx().mk_clauses_from_iter(binder_predicates.into_iter().chain(
                            additional_binder_predicates.into_iter().map(|(clause, _)| clause),
                        ));

                    let _ = self.instantiate_poly_trait_ref(
                        &poly_trait_ref.trait_ref,
                        poly_trait_ref.span,
                        constness,
                        polarity,
                        param_ty,
                        bounds,
                        false,
                        binder_predicates,
                        only_self_bounds,
                    );
                }
                hir::GenericBound::Outlives(lifetime) => {
                    let region = self.ast_region_to_region(lifetime, None);
                    bounds.push_region_bound(
                        self.tcx(),
                        ty::Binder::bind_with_vars(
                            ty::OutlivesPredicate(param_ty, region),
                            bound_vars,
                        ),
                        lifetime.ident.span,
                    );
                }
            }
        }
    }

    /// Translates a list of bounds from the HIR into the `Bounds` data structure.
    /// The self-type for the bounds is given by `param_ty`.
    ///
    /// Example:
    ///
    /// ```ignore (illustrative)
    /// fn foo<T: Bar + Baz>() { }
    /// //     ^  ^^^^^^^^^ ast_bounds
    /// //     param_ty
    /// ```
    ///
    /// The `sized_by_default` parameter indicates if, in this context, the `param_ty` should be
    /// considered `Sized` unless there is an explicit `?Sized` bound. This would be true in the
    /// example above, but is not true in supertrait listings like `trait Foo: Bar + Baz`.
    ///
    /// `span` should be the declaration size of the parameter.
    pub(crate) fn compute_bounds(
        &self,
        param_ty: Ty<'tcx>,
        ast_bounds: &[hir::GenericBound<'_>],
        filter: PredicateFilter,
    ) -> Bounds<'tcx> {
        let mut bounds = Bounds::default();

        let only_self_bounds = match filter {
            PredicateFilter::All | PredicateFilter::SelfAndAssociatedTypeBounds => {
                OnlySelfBounds(false)
            }
            PredicateFilter::SelfOnly | PredicateFilter::SelfThatDefines(_) => OnlySelfBounds(true),
        };

        self.add_bounds(
            param_ty,
            ast_bounds.iter().filter(|bound| match filter {
                PredicateFilter::All
                | PredicateFilter::SelfOnly
                | PredicateFilter::SelfAndAssociatedTypeBounds => true,
                PredicateFilter::SelfThatDefines(assoc_name) => {
                    if let Some(trait_ref) = bound.trait_ref()
                        && let Some(trait_did) = trait_ref.trait_def_id()
                        && self.tcx().trait_may_define_assoc_item(trait_did, assoc_name)
                    {
                        true
                    } else {
                        false
                    }
                }
            }),
            &mut bounds,
            ty::List::empty(),
            ty::List::empty(),
            only_self_bounds,
        );
        debug!(?bounds);

        bounds
    }

    /// Given an HIR binding like `Item = Foo` or `Item: Foo`, pushes the corresponding predicates
    /// onto `bounds`.
    ///
    /// **A note on binders:** given something like `T: for<'a> Iterator<Item = &'a u32>`, the
    /// `trait_ref` here will be `for<'a> T: Iterator`. The `binding` data however is from *inside*
    /// the binder (e.g., `&'a u32`) and hence may reference bound regions.
    #[instrument(level = "debug", skip(self, bounds, speculative, dup_bindings, path_span))]
    pub(super) fn add_predicates_for_ast_type_binding(
        &self,
        hir_ref_id: hir::HirId,
        trait_ref: ty::PolyTraitRef<'tcx>,
        binding: &ConvertedBinding<'_, 'tcx>,
        bounds: &mut Bounds<'tcx>,
        speculative: bool,
        dup_bindings: &mut FxHashMap<DefId, Span>,
        path_span: Span,
        constness: ty::BoundConstness,
        only_self_bounds: OnlySelfBounds,
        polarity: ty::ImplPolarity,
    ) -> Result<(), ErrorGuaranteed> {
        // Given something like `U: SomeTrait<T = X>`, we want to produce a
        // predicate like `<U as SomeTrait>::T = X`. This is somewhat
        // subtle in the event that `T` is defined in a supertrait of
        // `SomeTrait`, because in that case we need to upcast.
        //
        // That is, consider this case:
        //
        // ```
        // trait SubTrait: SuperTrait<i32> { }
        // trait SuperTrait<A> { type T; }
        //
        // ... B: SubTrait<T = foo> ...
        // ```
        //
        // We want to produce `<B as SuperTrait<i32>>::T == foo`.

        let tcx = self.tcx();

        let assoc_kind =
            if binding.gen_args.parenthesized == hir::GenericArgsParentheses::ReturnTypeNotation {
                ty::AssocKind::Fn
            } else if let ConvertedBindingKind::Equality(term) = binding.kind
                && let ty::TermKind::Const(_) = term.node.unpack()
            {
                ty::AssocKind::Const
            } else {
                ty::AssocKind::Type
            };

        let candidate = if self.trait_defines_associated_item_named(
            trait_ref.def_id(),
            assoc_kind,
            binding.item_name,
        ) {
            // Simple case: The assoc item is defined in the current trait.
            trait_ref
        } else {
            // Otherwise, we have to walk through the supertraits to find
            // one that does define it.
            self.one_bound_for_assoc_item(
                || traits::supertraits(tcx, trait_ref),
                trait_ref.skip_binder().print_only_trait_name(),
                None,
                assoc_kind,
                binding.item_name,
                path_span,
                Some(&binding),
            )?
        };

        let (assoc_ident, def_scope) =
            tcx.adjust_ident_and_get_scope(binding.item_name, candidate.def_id(), hir_ref_id);

        // We have already adjusted the item name above, so compare with `.normalize_to_macros_2_0()`
        // instead of calling `filter_by_name_and_kind` which would needlessly normalize the
        // `assoc_ident` again and again.
        let assoc_item = tcx
            .associated_items(candidate.def_id())
            .filter_by_name_unhygienic(assoc_ident.name)
            .find(|i| i.kind == assoc_kind && i.ident(tcx).normalize_to_macros_2_0() == assoc_ident)
            .expect("missing associated item");

        if !assoc_item.visibility(tcx).is_accessible_from(def_scope, tcx) {
            tcx.sess
                .struct_span_err(
                    binding.span,
                    format!("{} `{}` is private", assoc_item.kind, binding.item_name),
                )
                .span_label(binding.span, format!("private {}", assoc_item.kind))
                .emit();
        }
        tcx.check_stability(assoc_item.def_id, Some(hir_ref_id), binding.span, None);

        if !speculative {
            dup_bindings
                .entry(assoc_item.def_id)
                .and_modify(|prev_span| {
                    tcx.sess.emit_err(errors::ValueOfAssociatedStructAlreadySpecified {
                        span: binding.span,
                        prev_span: *prev_span,
                        item_name: binding.item_name,
                        def_path: tcx.def_path_str(assoc_item.container_id(tcx)),
                    });
                })
                .or_insert(binding.span);
        }

        let projection_ty = if let ty::AssocKind::Fn = assoc_kind {
            let mut emitted_bad_param_err = false;
            // If we have an method return type bound, then we need to substitute
            // the method's early bound params with suitable late-bound params.
            let mut num_bound_vars = candidate.bound_vars().len();
            let args =
                candidate.skip_binder().args.extend_to(tcx, assoc_item.def_id, |param, _| {
                    let subst = match param.kind {
                        ty::GenericParamDefKind::Lifetime => ty::Region::new_bound(
                            tcx,
                            ty::INNERMOST,
                            ty::BoundRegion {
                                var: ty::BoundVar::from_usize(num_bound_vars),
                                kind: ty::BoundRegionKind::BrNamed(param.def_id, param.name),
                            },
                        )
                        .into(),
                        ty::GenericParamDefKind::Type { .. } => {
                            if !emitted_bad_param_err {
                                tcx.sess.emit_err(
                                    crate::errors::ReturnTypeNotationIllegalParam::Type {
                                        span: path_span,
                                        param_span: tcx.def_span(param.def_id),
                                    },
                                );
                                emitted_bad_param_err = true;
                            }
                            Ty::new_bound(
                                tcx,
                                ty::INNERMOST,
                                ty::BoundTy {
                                    var: ty::BoundVar::from_usize(num_bound_vars),
                                    kind: ty::BoundTyKind::Param(param.def_id, param.name),
                                },
                            )
                            .into()
                        }
                        ty::GenericParamDefKind::Const { .. } => {
                            if !emitted_bad_param_err {
                                tcx.sess.emit_err(
                                    crate::errors::ReturnTypeNotationIllegalParam::Const {
                                        span: path_span,
                                        param_span: tcx.def_span(param.def_id),
                                    },
                                );
                                emitted_bad_param_err = true;
                            }
                            let ty = tcx
                                .type_of(param.def_id)
                                .no_bound_vars()
                                .expect("ct params cannot have early bound vars");
                            ty::Const::new_bound(
                                tcx,
                                ty::INNERMOST,
                                ty::BoundVar::from_usize(num_bound_vars),
                                ty,
                            )
                            .into()
                        }
                    };
                    num_bound_vars += 1;
                    subst
                });

            // Next, we need to check that the return-type notation is being used on
            // an RPITIT (return-position impl trait in trait) or AFIT (async fn in trait).
            let output = tcx.fn_sig(assoc_item.def_id).skip_binder().output();
            let output = if let ty::Alias(ty::Projection, alias_ty) = *output.skip_binder().kind()
                && tcx.is_impl_trait_in_trait(alias_ty.def_id)
            {
                alias_ty
            } else {
                return Err(self.tcx().sess.emit_err(
                    crate::errors::ReturnTypeNotationOnNonRpitit {
                        span: binding.span,
                        ty: tcx.liberate_late_bound_regions(assoc_item.def_id, output),
                        fn_span: tcx.hir().span_if_local(assoc_item.def_id),
                        note: (),
                    },
                ));
            };

            // Finally, move the fn return type's bound vars over to account for the early bound
            // params (and trait ref's late bound params). This logic is very similar to
            // `Predicate::subst_supertrait`, and it's no coincidence why.
            let shifted_output = tcx.shift_bound_var_indices(num_bound_vars, output);
            let subst_output = ty::EarlyBinder::bind(shifted_output).instantiate(tcx, args);

            let bound_vars = tcx.late_bound_vars(binding.hir_id);
            ty::Binder::bind_with_vars(subst_output, bound_vars)
        } else {
            // Append the generic arguments of the associated type to the `trait_ref`.
            candidate.map_bound(|trait_ref| {
                let ident = Ident::new(assoc_item.name, binding.item_name.span);
                let item_segment = hir::PathSegment {
                    ident,
                    hir_id: binding.hir_id,
                    res: Res::Err,
                    args: Some(binding.gen_args),
                    infer_args: false,
                };

                let args_trait_ref_and_assoc_item = self.create_args_for_associated_item(
                    path_span,
                    assoc_item.def_id,
                    &item_segment,
                    trait_ref.args,
                );

                debug!(?args_trait_ref_and_assoc_item);

                ty::AliasTy::new(tcx, assoc_item.def_id, args_trait_ref_and_assoc_item)
            })
        };

        if !speculative {
            // Find any late-bound regions declared in `ty` that are not
            // declared in the trait-ref or assoc_item. These are not well-formed.
            //
            // Example:
            //
            //     for<'a> <T as Iterator>::Item = &'a str // <-- 'a is bad
            //     for<'a> <T as FnMut<(&'a u32,)>>::Output = &'a str // <-- 'a is ok
            if let ConvertedBindingKind::Equality(ty) = binding.kind {
                let late_bound_in_trait_ref =
                    tcx.collect_constrained_late_bound_regions(&projection_ty);
                let late_bound_in_ty =
                    tcx.collect_referenced_late_bound_regions(&trait_ref.rebind(ty.node));
                debug!(?late_bound_in_trait_ref);
                debug!(?late_bound_in_ty);

                // FIXME: point at the type params that don't have appropriate lifetimes:
                // struct S1<F: for<'a> Fn(&i32, &i32) -> &'a i32>(F);
                //                         ----  ----     ^^^^^^^
                self.validate_late_bound_regions(
                    late_bound_in_trait_ref,
                    late_bound_in_ty,
                    |br_name| {
                        struct_span_err!(
                            tcx.sess,
                            binding.span,
                            E0582,
                            "binding for associated type `{}` references {}, \
                             which does not appear in the trait input types",
                            binding.item_name,
                            br_name
                        )
                    },
                );
            }
        }

        match binding.kind {
            ConvertedBindingKind::Equality(..) if let ty::AssocKind::Fn = assoc_kind => {
                return Err(self.tcx().sess.emit_err(
                    crate::errors::ReturnTypeNotationEqualityBound { span: binding.span },
                ));
            }
            ConvertedBindingKind::Equality(term) => {
                // "Desugar" a constraint like `T: Iterator<Item = u32>` this to
                // the "projection predicate" for:
                //
                // `<T as Iterator>::Item = u32`
                bounds.push_projection_bound(
                    tcx,
                    projection_ty.map_bound(|projection_ty| ty::ProjectionPredicate {
                        projection_ty,
                        term: term.node,
                    }),
                    binding.span,
                );
            }
            ConvertedBindingKind::Constraint(ast_bounds) => {
                // "Desugar" a constraint like `T: Iterator<Item: Debug>` to
                //
                // `<T as Iterator>::Item: Debug`
                //
                // Calling `skip_binder` is okay, because `add_bounds` expects the `param_ty`
                // parameter to have a skipped binder.
                //
                // NOTE: If `only_self_bounds` is true, do NOT expand this associated
                // type bound into a trait predicate, since we only want to add predicates
                // for the `Self` type.
                if !only_self_bounds.0 {
                    let param_ty = Ty::new_alias(tcx, ty::Projection, projection_ty.skip_binder());
                    self.add_bounds(
                        param_ty,
                        ast_bounds.iter(),
                        bounds,
                        projection_ty.bound_vars(),
                        projection_ty.skip_binder_with_predicates().1,
                        only_self_bounds,
                    );
                }
            }
        }
        Ok(())
    }
}

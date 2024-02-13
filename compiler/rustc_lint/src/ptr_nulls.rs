use crate::{
    lints::{InvalidNullPtrUsagesDiag, InvalidNullPtrUsagesSuggestion, UselessPtrNullChecksDiag},
    reference_casting::peel_casts,
    LateContext, LateLintPass, LintContext,
};
use rustc_ast::LitKind;
use rustc_hir::{BinOpKind, Expr, ExprKind, TyKind};
use rustc_middle::ty::{RawPtr, TypeAndMut};
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::sym;

declare_lint! {
    /// The `useless_ptr_null_checks` lint checks for useless null checks against pointers
    /// obtained from non-null types.
    ///
    /// ### Example
    ///
    /// ```rust
    /// # fn test() {}
    /// let fn_ptr: fn() = /* somehow obtained nullable function pointer */
    /// #   test;
    ///
    /// if (fn_ptr as *const ()).is_null() { /* ... */ }
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Function pointers and references are assumed to be non-null, checking them for null
    /// will always return false.
    USELESS_PTR_NULL_CHECKS,
    Warn,
    "useless checking of non-null-typed pointer"
}

declare_lint! {
    /// The `invalid_null_ptr_usages` lint checks for invalid usage of null pointers.
    ///
    /// ### Example
    ///
    /// ```rust,compile_fail
    /// # use std::{slice, ptr};
    /// // Undefined behavior
    /// # let _slice: &[u8] =
    /// unsafe { slice::from_raw_parts(ptr::null(), 0) };
    /// ```
    ///
    /// {{produces}}
    ///
    /// ### Explanation
    ///
    /// Calling methods whos safety invariants requires non-null ptr with a null-ptr
    /// is undefined behavior.
    INVALID_NULL_PTR_USAGES,
    Deny,
    "invalid call with null ptr"
}

declare_lint_pass!(PtrNullChecks => [USELESS_PTR_NULL_CHECKS, INVALID_NULL_PTR_USAGES]);

/// This function checks if the expression is from a series of consecutive casts,
/// ie. `(my_fn as *const _ as *mut _).cast_mut()` and whether the original expression is either
/// a fn ptr, a reference, or a function call whose definition is
/// annotated with `#![rustc_never_returns_null_ptr]`.
/// If this situation is present, the function returns the appropriate diagnostic.
fn useless_check<'a, 'tcx: 'a>(
    cx: &'a LateContext<'tcx>,
    mut e: &'a Expr<'a>,
) -> Option<UselessPtrNullChecksDiag<'tcx>> {
    let mut had_at_least_one_cast = false;
    loop {
        e = e.peel_blocks();
        if let ExprKind::MethodCall(_, _expr, [], _) = e.kind
            && let Some(def_id) = cx.typeck_results().type_dependent_def_id(e.hir_id)
            && cx.tcx.has_attr(def_id, sym::rustc_never_returns_null_ptr)
            && let Some(fn_name) = cx.tcx.opt_item_ident(def_id)
        {
            return Some(UselessPtrNullChecksDiag::FnRet { fn_name });
        } else if let ExprKind::Call(path, _args) = e.kind
            && let ExprKind::Path(ref qpath) = path.kind
            && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
            && cx.tcx.has_attr(def_id, sym::rustc_never_returns_null_ptr)
            && let Some(fn_name) = cx.tcx.opt_item_ident(def_id)
        {
            return Some(UselessPtrNullChecksDiag::FnRet { fn_name });
        }
        e = if let ExprKind::Cast(expr, t) = e.kind
            && let TyKind::Ptr(_) = t.kind
        {
            had_at_least_one_cast = true;
            expr
        } else if let ExprKind::MethodCall(_, expr, [], _) = e.kind
            && let Some(def_id) = cx.typeck_results().type_dependent_def_id(e.hir_id)
            && matches!(cx.tcx.get_diagnostic_name(def_id), Some(sym::ptr_cast | sym::ptr_cast_mut))
        {
            had_at_least_one_cast = true;
            expr
        } else if had_at_least_one_cast {
            let orig_ty = cx.typeck_results().expr_ty(e);
            return if orig_ty.is_fn() {
                Some(UselessPtrNullChecksDiag::FnPtr { orig_ty, label: e.span })
            } else if orig_ty.is_ref() {
                Some(UselessPtrNullChecksDiag::Ref { orig_ty, label: e.span })
            } else {
                None
            };
        } else {
            return None;
        };
    }
}

fn is_null_ptr<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> bool {
    let (expr, _) = peel_casts(cx, expr);

    if let ExprKind::Call(path, []) = expr.kind
        && let ExprKind::Path(ref qpath) = path.kind
        && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
        && let Some(diag_item) = cx.tcx.get_diagnostic_name(def_id)
    {
        diag_item == sym::ptr_null || diag_item == sym::ptr_null_mut
    } else if let ExprKind::Lit(spanned) = expr.kind
        && let LitKind::Int(v, _) = spanned.node
    {
        v == 0
    } else {
        false
    }
}

impl<'tcx> LateLintPass<'tcx> for PtrNullChecks {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        match expr.kind {
            // Catching:
            // <*<const/mut> <ty>>::is_null(fn_ptr as *<const/mut> <ty>)
            ExprKind::Call(path, [arg])
                if let ExprKind::Path(ref qpath) = path.kind
                    && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
                    && matches!(
                        cx.tcx.get_diagnostic_name(def_id),
                        Some(sym::ptr_const_is_null | sym::ptr_is_null)
                    )
                    && let Some(diag) = useless_check(cx, arg) =>
            {
                cx.emit_span_lint(USELESS_PTR_NULL_CHECKS, expr.span, diag)
            }

            // Catching:
            // <path>(arg...) where `arg` is null-ptr and `path` is a fn that expect non-null-ptr
            ExprKind::Call(path, args)
                if let ExprKind::Path(ref qpath) = path.kind
                    && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
                    && let Some(diag_name) = cx.tcx.get_diagnostic_name(def_id) =>
            {
                // `arg` positions where null would cause U.B.
                let arg_indices: &[_] = match diag_name {
                    sym::ptr_read
                    | sym::ptr_read_unaligned
                    | sym::ptr_read_volatile
                    | sym::ptr_replace
                    | sym::ptr_write
                    | sym::ptr_write_bytes
                    | sym::ptr_write_unaligned
                    | sym::ptr_write_volatile
                    | sym::slice_from_raw_parts
                    | sym::slice_from_raw_parts_mut => &[0],
                    sym::ptr_copy
                    | sym::ptr_copy_nonoverlapping
                    | sym::ptr_swap
                    | sym::ptr_swap_nonoverlapping => &[0, 1],
                    _ => return,
                };

                for &arg_idx in arg_indices {
                    if let Some(arg) = args.get(arg_idx).filter(|arg| is_null_ptr(cx, arg)) {
                        let arg_span = arg.span;

                        let suggestion = if let ExprKind::Cast(..) = arg.peel_blocks().kind
                            && let Some(ty) = cx.typeck_results().expr_ty_opt(arg)
                            && let RawPtr(TypeAndMut { ty, .. }) = ty.kind()
                        {
                            InvalidNullPtrUsagesSuggestion::WithExplicitType { ty: *ty, arg_span }
                        } else {
                            InvalidNullPtrUsagesSuggestion::WithoutExplicitType { arg_span }
                        };

                        cx.emit_spanned_lint(
                            INVALID_NULL_PTR_USAGES,
                            expr.span,
                            InvalidNullPtrUsagesDiag { suggestion },
                        )
                    }
                }
            }

            // Catching:
            // (fn_ptr as *<const/mut> <ty>).is_null()
            ExprKind::MethodCall(_, receiver, _, _)
                if let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
                    && matches!(
                        cx.tcx.get_diagnostic_name(def_id),
                        Some(sym::ptr_const_is_null | sym::ptr_is_null)
                    )
                    && let Some(diag) = useless_check(cx, receiver) =>
            {
                cx.emit_span_lint(USELESS_PTR_NULL_CHECKS, expr.span, diag)
            }

            ExprKind::Binary(op, left, right) if matches!(op.node, BinOpKind::Eq) => {
                let to_check: &Expr<'_>;
                let diag: UselessPtrNullChecksDiag<'_>;
                if let Some(ddiag) = useless_check(cx, left) {
                    to_check = right;
                    diag = ddiag;
                } else if let Some(ddiag) = useless_check(cx, right) {
                    to_check = left;
                    diag = ddiag;
                } else {
                    return;
                }

                match to_check.kind {
                    // Catching:
                    // (fn_ptr as *<const/mut> <ty>) == (0 as <ty>)
                    ExprKind::Cast(cast_expr, _)
                        if let ExprKind::Lit(spanned) = cast_expr.kind
                            && let LitKind::Int(v, _) = spanned.node
                            && v == 0 =>
                    {
                        cx.emit_span_lint(USELESS_PTR_NULL_CHECKS, expr.span, diag)
                    }

                    // Catching:
                    // (fn_ptr as *<const/mut> <ty>) == std::ptr::null()
                    ExprKind::Call(path, [])
                        if let ExprKind::Path(ref qpath) = path.kind
                            && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
                            && let Some(diag_item) = cx.tcx.get_diagnostic_name(def_id)
                            && (diag_item == sym::ptr_null || diag_item == sym::ptr_null_mut) =>
                    {
                        cx.emit_span_lint(USELESS_PTR_NULL_CHECKS, expr.span, diag)
                    }

                    _ => {}
                }
            }
            _ => {}
        }
    }
}

use rustc_ast::token;
use rustc_ast::tokenstream::{DelimSpacing, DelimSpan, Spacing, TokenStream, TokenTree};
use rustc_errors::ErrorGuaranteed;
use rustc_expand::base::{AttrProcMacro, ExtCtxt};
use rustc_span::symbol::{sym, Symbol};
use rustc_span::Span;

pub struct ExpandRequires;

impl AttrProcMacro for ExpandRequires {
    fn expand<'cx>(
        &self,
        ecx: &'cx mut ExtCtxt<'_>,
        span: Span,
        annotation: TokenStream,
        annotated: TokenStream,
    ) -> Result<TokenStream, ErrorGuaranteed> {
        expand_requires_tts(ecx, span, annotation, annotated)
    }
}

fn expand_requires_tts(
    _ecx: &mut ExtCtxt<'_>,
    attr_span: Span,
    annotation: TokenStream,
    annotated: TokenStream,
) -> Result<TokenStream, ErrorGuaranteed> {
    let mut new_tts = Vec::with_capacity(annotated.len());
    let mut cursor = annotated.into_trees();

    // NOTE: this will break if you have a `{ ... }` syntactically prior to the fn body,
    // e.g. if a `const { ... }` can occur in a where clause. We need to do something
    // smarter for injecting this code into the right place.
    while let Some(tt) = cursor.next_ref() {
        if let TokenTree::Delimited(sp, spacing, delim @ token::Delimiter::Brace, inner_ts) = tt {
            let token_for_chain = |tok| std::iter::once(TokenTree::token_alone(tok, attr_span));
            let ident_for_chain = |sym| token_for_chain(token::Ident(sym, token::IdentIsRaw::No));

            let intrinsic_call_arguments = TokenTree::Delimited(
                DelimSpan::from_single(attr_span),
                DelimSpacing { open: Spacing::JointHidden, close: Spacing::JointHidden },
                token::Delimiter::Parenthesis,
                TokenStream::new(
                    token_for_chain(token::BinOp(token::BinOpToken::Or))
                        .chain(token_for_chain(token::BinOp(token::BinOpToken::Or)))
                        .chain(std::iter::once(TokenTree::Delimited(
                            DelimSpan::from_single(attr_span),
                            DelimSpacing {
                                open: Spacing::JointHidden,
                                close: Spacing::JointHidden,
                            },
                            token::Delimiter::Brace,
                            annotation,
                        )))
                        .chain(token_for_chain(token::Comma))
                        .chain(token_for_chain(token::Literal(token::Lit::new(
                            token::LitKind::Str,
                            Symbol::intern("contract failure"),
                            None,
                        ))))
                        .collect(),
                ),
            );

            let revised_tt = TokenTree::Delimited(
                *sp,
                *spacing,
                *delim,
                token_for_chain(token::Ident(sym::core, token::IdentIsRaw::No))
                    .chain(token_for_chain(token::PathSep))
                    .chain(ident_for_chain(sym::intrinsics))
                    .chain(token_for_chain(token::PathSep))
                    .chain(ident_for_chain(sym::contract_check))
                    .chain(std::iter::once(intrinsic_call_arguments))
                    .chain(token_for_chain(token::Semi))
                    .chain(inner_ts.trees().cloned())
                    .collect(),
            );
            new_tts.push(revised_tt);
            break;
        } else {
            new_tts.push(tt.clone());
            continue;
        }
    }

    // Above we injected the intrinsic call. Now just copy over all the other token trees.
    while let Some(tt) = cursor.next_ref() {
        new_tts.push(tt.clone());
    }
    Ok(TokenStream::new(new_tts))
}

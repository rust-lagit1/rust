use rustc_ast::token;
use rustc_ast::tokenstream::{DelimSpan, DelimSpacing, Spacing, TokenStream, TokenTree};
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

    // XXX this will break if you have a `{ ... }` syntactically prior to the fn body,
    // e.g. if a `const { ... }` can occur in a where clause. We need to do something
    // smarter for injecting this code into the right place.
    while let Some(tt) = cursor.next_ref() {
        if let TokenTree::Delimited(sp, spacing, delim @ token::Delimiter::Brace, inner_ts) = tt {
            let revised_tt = TokenTree::Delimited(
                *sp, *spacing, *delim,

                // XXX a constructed ast doesn't actually *carry*
                // tokens from which to  build a TokenStream.
                // So instead, use the original annotation directly.
                // TokenStream::from_ast(&ast_for_invoke)

                std::iter::once(TokenTree::token_alone(token::Ident(sym::core, token::IdentIsRaw::No), attr_span))
                    .chain(std::iter::once(TokenTree::token_alone(token::PathSep, attr_span)))
                    .chain(std::iter::once(TokenTree::token_alone(token::Ident(sym::intrinsics, token::IdentIsRaw::No), attr_span)))
                    .chain(std::iter::once(TokenTree::token_alone(token::PathSep, attr_span)))
                    .chain(std::iter::once(TokenTree::token_alone(token::Ident(sym::contract_check, token::IdentIsRaw::No), attr_span)))
                    .chain(std::iter::once(TokenTree::Delimited(
                        DelimSpan::from_single(attr_span),
                        DelimSpacing { open: Spacing::JointHidden, close: Spacing::JointHidden },
                        token::Delimiter::Parenthesis,
                        TokenStream::new(
                            std::iter::once(TokenTree::token_alone(token::BinOp(token::BinOpToken::Or), attr_span))
                                .chain(std::iter::once(TokenTree::token_alone(token::BinOp(token::BinOpToken::Or), attr_span)))
                                .chain(std::iter::once(TokenTree::Delimited(DelimSpan::from_single(attr_span),
                                                                            DelimSpacing { open: Spacing::JointHidden, close: Spacing::JointHidden },
                                                                            token::Delimiter::Brace,
                                                                            annotation)))
                                .chain(std::iter::once(TokenTree::token_alone(token::Comma, attr_span)))
                                .chain(std::iter::once(TokenTree::token_alone(token::Literal(token::Lit::new(token::LitKind::Str, Symbol::intern("contract failure"), None)), attr_span)))
                                .collect()
                        ))))
                    .chain(TokenStream::token_alone(token::Semi, attr_span).trees().cloned())
                    .chain(inner_ts.trees().cloned())
                    .collect());
            new_tts.push(revised_tt);
            break;
        } else {
            new_tts.push(tt.clone());
            continue;
        }
    };

    // Above we injected the intrinsic call. Now just copy over all the other token trees.
    while let Some(tt) = cursor.next_ref() {
        new_tts.push(tt.clone());
    }
    Ok(TokenStream::new(new_tts))
}

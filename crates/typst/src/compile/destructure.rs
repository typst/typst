use ecow::EcoString;
use smallvec::SmallVec;
use typst_syntax::{
    ast::{self, AstNode},
    Span,
};

use crate::diag::{bail, SourceResult};

use super::{compiler::Compiler, Access, AccessPattern, ScopeId};

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct Pattern {
    pub span: Span,
    pub kind: PatternKind,
}

pub trait PatternCompile {
    fn compile(&self, compiler: &mut Compiler, declare: bool) -> SourceResult<Pattern>;
}

impl PatternCompile for ast::Pattern<'_> {
    fn compile(&self, compiler: &mut Compiler, declare: bool) -> SourceResult<Pattern> {
        match self {
            ast::Pattern::Normal(normal) => match normal {
                ast::Expr::Ident(ident) => {
                    let index = if declare {
                        let id = compiler.local(ident.span(), ident.get().clone());
                        AccessPattern::Local(ScopeId::SELF, id)
                    } else {
                        ident.access(compiler, false)?
                    };

                    index.free(compiler);

                    return Ok(Pattern {
                        span: ident.span(),
                        kind: PatternKind::Single(PatternItem::Simple(
                            normal.span(),
                            index,
                            ident.get().clone(),
                        )),
                    });
                }
                _ => bail!(self.span(), "nested patterns are currently not supported"),
            },
            ast::Pattern::Placeholder(placeholder) => {
                return Ok(Pattern {
                    span: placeholder.span(),
                    kind: PatternKind::Single(PatternItem::Placeholder(
                        placeholder.span(),
                    )),
                })
            }
            ast::Pattern::Destructuring(destructure) => {
                let mut items = SmallVec::new();
                for binding in destructure.bindings() {
                    match binding {
                        ast::DestructuringKind::Normal(normal) => match normal {
                            ast::Expr::Ident(ident) => {
                                let index = if declare {
                                    let id =
                                        compiler.local(ident.span(), ident.get().clone());
                                    AccessPattern::Local(ScopeId::SELF, id)
                                } else {
                                    ident.access(compiler, false)?
                                };

                                index.free(compiler);

                                items.push(PatternItem::Simple(
                                    binding.span(),
                                    index,
                                    ident.get().clone(),
                                ));
                            }
                            _ => bail!(
                                self.span(),
                                "nested patterns are currently not supported"
                            ),
                        },
                        ast::DestructuringKind::Sink(sink) => {
                            if let Some(ident) = sink.name() {
                                let index = if declare {
                                    let id =
                                        compiler.local(ident.span(), ident.get().clone());
                                    AccessPattern::Local(ScopeId::SELF, id)
                                } else {
                                    ident.access(compiler, false)?
                                };

                                index.free(compiler);

                                items.push(PatternItem::Spread(sink.span(), index));
                            } else {
                                items.push(PatternItem::SpreadDiscard(sink.span()));
                            }
                        }
                        ast::DestructuringKind::Named(named) => {
                            let index = if let ast::Expr::Ident(ident) = named.expr() {
                                let id =
                                    compiler.local(ident.span(), ident.get().clone());
                                AccessPattern::Local(ScopeId::SELF, id)
                            } else if declare {
                                bail!(
                                    named.expr().span(),
                                    "cannot declare a named pattern"
                                );
                            } else {
                                named.expr().access(compiler, false)?
                            };

                            index.free(compiler);
                            items.push(PatternItem::Named(
                                named.span(),
                                index,
                                named.name().get().clone(),
                            ));
                        }
                        ast::DestructuringKind::Placeholder(placeholder) => {
                            items.push(PatternItem::Placeholder(placeholder.span()))
                        }
                    }
                }

                Ok(Pattern {
                    span: destructure.span(),
                    kind: PatternKind::Tuple(items),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum PatternKind {
    /// Destructure into a single local.
    Single(PatternItem),

    /// Destructure into a tuple of locals.
    Tuple(SmallVec<[PatternItem; 2]>),
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum PatternItem {
    /// Destructure into a single local.
    Placeholder(Span),

    /// Destructure into a single local.
    Simple(Span, AccessPattern, EcoString),

    /// Spread the remaining values into a single value.
    Spread(Span, AccessPattern),

    /// Spread the remaining values into a single value and discard it.
    SpreadDiscard(Span),

    /// A named pattern.
    Named(Span, AccessPattern, EcoString),
}

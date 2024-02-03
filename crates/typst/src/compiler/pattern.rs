use ecow::EcoString;
use smallvec::SmallVec;
use typst_syntax::{
    ast::{self, AstNode},
    Span,
};

use crate::vm::{
    Pattern as VmPattern, PatternItem as VmPatternItem, PatternKind as VmPatternKind,
};
use crate::{
    diag::{bail, At, SourceResult},
    engine::Engine,
};

use super::{Access, AccessPattern, Compiler};

#[derive(Debug, Clone)]
pub struct Pattern {
    pub span: Span,
    pub kind: PatternKind,
}

impl Pattern {
    pub fn as_vm_pattern(&self) -> VmPattern {
        VmPattern { span: self.span, kind: self.kind.as_vm_kind() }
    }
}

pub trait PatternCompile {
    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        declare: bool,
    ) -> SourceResult<Pattern>;
}

impl PatternCompile for ast::Pattern<'_> {
    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        declare: bool,
    ) -> SourceResult<Pattern> {
        match self {
            ast::Pattern::Normal(normal) => match normal {
                ast::Expr::Ident(ident) => {
                    let index = if declare {
                        let id = compiler
                            .declare(ident.span(), ident.get().clone())
                            .at(self.span())?;
                        AccessPattern::Writable(id.into())
                    } else {
                        ident.access(engine, compiler, false)?
                    };

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
                                    let id = compiler
                                        .declare(ident.span(), ident.get().clone())
                                        .at(self.span())?;
                                    AccessPattern::Writable(id.into())
                                } else {
                                    ident.access(engine, compiler, false)?
                                };

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
                                    let id = compiler
                                        .declare(ident.span(), ident.get().clone())
                                        .at(self.span())?;
                                    AccessPattern::Writable(id.into())
                                } else {
                                    ident.access(engine, compiler, false)?
                                };

                                items.push(PatternItem::Spread(sink.span(), index));
                            } else {
                                items.push(PatternItem::SpreadDiscard(sink.span()));
                            }
                        }
                        ast::DestructuringKind::Named(named) => {
                            let index = if let ast::Expr::Ident(ident) = named.expr() {
                                let id = compiler
                                    .declare(ident.span(), ident.get().clone())
                                    .at(self.span())?;
                                AccessPattern::Writable(id.into())
                            } else if declare {
                                bail!(
                                    named.expr().span(),
                                    "cannot declare a named pattern"
                                );
                            } else {
                                named.expr().access(engine, compiler, false)?
                            };

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

#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Destructure into a single local.
    Single(PatternItem),

    /// Destructure into a tuple of locals.
    Tuple(SmallVec<[PatternItem; 2]>),
}

impl PatternKind {
    pub fn as_vm_kind(&self) -> VmPatternKind {
        match self {
            Self::Single(item) => VmPatternKind::Single(item.as_vm_item()),
            Self::Tuple(items) => {
                VmPatternKind::Tuple(items.iter().map(|item| item.as_vm_item()).collect())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum PatternItem {
    /// Destructure into nothing.
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

impl PatternItem {
    fn as_vm_item(&self) -> VmPatternItem {
        match self {
            Self::Placeholder(span) => VmPatternItem::Placeholder(*span),
            Self::Simple(span, access, name) => {
                VmPatternItem::Simple(*span, access.as_vm_access(), name.clone())
            }
            Self::Spread(span, access) => {
                VmPatternItem::Spread(*span, access.as_vm_access())
            }
            Self::SpreadDiscard(span) => VmPatternItem::SpreadDiscard(*span),
            Self::Named(span, access, name) => {
                VmPatternItem::Named(*span, access.as_vm_access(), name.clone())
            }
        }
    }
}

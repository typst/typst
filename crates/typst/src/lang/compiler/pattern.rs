use smallvec::SmallVec;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::lang::compiled::{CompiledPattern, CompiledPatternItem, CompiledPatternKind};
use crate::lang::opcodes::AccessId;
use crate::lang::operands::{PatternId, StringId};

use super::access::{Access, CompileAccess};
use super::{Compiler, IntoCompiledValue};

#[derive(Debug, Clone, Hash)]
pub struct Pattern {
    pub span: Span,
    pub kind: PatternKind,
}

impl IntoCompiledValue for Pattern {
    type CompiledValue = CompiledPattern;

    fn into_compiled_value(self) -> Self::CompiledValue {
        CompiledPattern {
            span: self.span,
            kind: self.kind.into_compiled_value(),
        }
    }
}

pub trait PatternCompile {
    fn compile_pattern(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        declare: bool,
    ) -> SourceResult<Pattern>;
}

impl PatternCompile for ast::Pattern<'_> {
    fn compile_pattern(
        &self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        declare: bool,
    ) -> SourceResult<Pattern> {
        match self {
            ast::Pattern::Parenthesized(paren) => {
                paren.pattern().compile_pattern(compiler, engine, declare)
            }
            ast::Pattern::Normal(normal) => match normal {
                ast::Expr::Ident(ident) => {
                    let access = if declare {
                        let id = compiler.declare(ident.span(), ident.get().as_str());
                        Access::Writable(id)
                    } else {
                        ident.access(compiler, engine, false)?
                    };

                    let access_id = compiler.access(access);
                    let name_id = compiler.string(ident.as_str());
                    Ok(Pattern {
                        span: ident.span(),
                        kind: PatternKind::Single(PatternItem::Simple(
                            normal.span(),
                            access_id,
                            name_id,
                        )),
                    })
                }
                _ => bail!(self.span(), "nested patterns are currently not supported"),
            },
            ast::Pattern::Placeholder(placeholder) => {
                Ok(Pattern {
                    span: placeholder.span(),
                    kind: PatternKind::Single(PatternItem::Placeholder(
                        placeholder.span(),
                    )),
                })
            }
            ast::Pattern::Destructuring(destructure) => {
                let mut items = SmallVec::new();
                let mut has_sink = false;
                for binding in destructure.items() {
                    match binding {
                        // Shorthand for a direct identifier.
                        ast::DestructuringItem::Pattern(ast::Pattern::Normal(
                            ast::Expr::Ident(ident),
                        )) => {
                            let access = if declare {
                                let id =
                                    compiler.declare(ident.span(), ident.get().as_str());
                                Access::Writable(id)
                            } else {
                                ident.access(compiler, engine, false)?
                            };

                            let access_id = compiler.access(access);
                            let name_id = compiler.string(ident.as_str());
                            items.push(PatternItem::Simple(
                                binding.span(),
                                access_id,
                                name_id,
                            ));
                        }
                        ast::DestructuringItem::Pattern(pattern) => {
                            let pattern =
                                pattern.compile_pattern(compiler, engine, declare)?;
                            let index = compiler.pattern(pattern);
                            items.push(PatternItem::Nested(binding.span(), index));
                        }
                        ast::DestructuringItem::Spread(spread) => {
                            if let Some(ident) = spread.sink_ident() {
                                has_sink = true;
                                let access = if declare {
                                    let id = compiler
                                        .declare(ident.span(), ident.get().as_str());
                                    Access::Writable(id)
                                } else {
                                    ident.access(compiler, engine, false)?
                                };

                                let access_id = compiler.access(access);
                                items.push(PatternItem::Spread(spread.span(), access_id));
                            } else {
                                items.push(PatternItem::SpreadDiscard(spread.span()));
                            }
                        }
                        ast::DestructuringItem::Named(named) => {
                            let access = if let ast::Expr::Ident(ident) = named.expr() {
                                let id =
                                    compiler.declare(ident.span(), ident.get().as_str());
                                Access::Writable(id)
                            } else if declare {
                                bail!(
                                    named.expr().span(),
                                    "cannot declare a named pattern"
                                );
                            } else {
                                named.expr().access(compiler, engine, false)?
                            };

                            let access_id = compiler.access(access);
                            let name_id = compiler.string(named.name().as_str());
                            items.push(PatternItem::Named(
                                named.span(),
                                access_id,
                                name_id,
                            ));
                        }
                    }
                }

                Ok(Pattern {
                    span: destructure.span(),
                    kind: PatternKind::Tuple(items, has_sink),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub enum PatternKind {
    /// Destructure into a single local.
    Single(PatternItem),

    /// Destructure into a tuple of locals.
    Tuple(SmallVec<[PatternItem; 2]>, bool),
}

impl IntoCompiledValue for PatternKind {
    type CompiledValue = CompiledPatternKind;

    fn into_compiled_value(self) -> Self::CompiledValue {
        match self {
            Self::Single(item) => CompiledPatternKind::Single(item.into_compiled_value()),
            Self::Tuple(items, has_sink) => CompiledPatternKind::Tuple(
                items
                    .into_iter()
                    .map(IntoCompiledValue::into_compiled_value)
                    .collect(),
                has_sink,
            ),
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub enum PatternItem {
    /// Destructure into nothing.
    Placeholder(Span),

    /// Destructure into a single local.
    Simple(Span, AccessId, StringId),

    /// Destructure into a nested pattern.
    Nested(Span, PatternId),

    /// Spread the remaining values into a single value.
    Spread(Span, AccessId),

    /// Spread the remaining values into a single value and discard it.
    SpreadDiscard(Span),

    /// A named pattern.
    Named(Span, AccessId, StringId),
}

impl IntoCompiledValue for PatternItem {
    type CompiledValue = CompiledPatternItem;

    fn into_compiled_value(self) -> Self::CompiledValue {
        match self {
            PatternItem::Placeholder(span) => CompiledPatternItem::Placeholder(span),
            PatternItem::Simple(span, access, name) => {
                CompiledPatternItem::Simple(span, access, name)
            }
            PatternItem::Nested(span, id) => CompiledPatternItem::Nested(span, id),
            PatternItem::Spread(span, access) => {
                CompiledPatternItem::Spread(span, access)
            }
            PatternItem::SpreadDiscard(span) => CompiledPatternItem::SpreadDiscard(span),
            PatternItem::Named(span, access, name) => {
                CompiledPatternItem::Named(span, access, name)
            }
        }
    }

    /*fn as_vm_item(&self) -> VmPatternItem {
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
    }*/
}

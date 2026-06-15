use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use typst_library::diag::{At, SourceDiagnostic, SourceResult, error, warning};
use typst_library::foundations::{
    Content, Func, NativeElement, Symbol, SymbolElem, Value,
};
use typst_library::math::{
    AlignPointElem, AttachElem, EquationElem, FracElem, LrElem, PrimesElem, RootElem,
};
use typst_library::text::TextElem;
use typst_syntax::ast::{self, AstNode, MathTextKind};
use typst_syntax::{DiagSpan, SubRange, SyntaxKind, SyntaxNode};

use crate::{Eval, Vm};

impl Eval for ast::Equation<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let body = self.body().eval(vm)?;
        let block = match self.block() {
            ast::EquationBlock::Consistent { block } => block,
            ast::EquationBlock::Inconsistent => {
                vm.engine.sink.warn(warning!(
                    self.span(),
                    "inconsistent spacing next to opening and closing dollar signs";
                    hint: "a block-level equation requires whitespace both after the \
                           opening dollar sign and before the closing dollar sign";
                    hint: "an inline equation should not have whitespace on either side";
                    hint: "this is being treated as an inline equation";
                ));
                // We treat inconsistently spaced equations as inline since one
                // of the sides didn't have a space. This avoids shifting the
                // layout when writing `$a + $` before typing `b`.
                false
            }
        };
        Ok(EquationElem::new(body).with_block(block).pack())
    }
}

impl Eval for ast::Math<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let mut expr_offsets = self.expr_offsets();
        let iter = std::iter::from_fn(move || {
            let (expr, expr_start) = expr_offsets.next()?;
            Some(expr.eval_display(vm).map_err(|math_error| {
                match math_error {
                    MathError::Normal(err) => err,
                    MathError::FuncLiteral { node, name } => {
                        // Add a custom hint if the error was due to a function
                        // literal followed by delimiters.
                        let mut overall_span = None;
                        let delims = expr_offsets
                            .find(|(expr, _)| !matches!(expr, ast::Expr::Space(_)))
                            .and_then(|(non_space, offset)| {
                                let ast::Expr::MathDelimited(delims) = non_space else {
                                    return None;
                                };
                                let end = offset + delims.to_untyped().len();
                                overall_span = Some(DiagSpan::from_span(
                                    self.span(),
                                    SubRange::new(expr_start, end),
                                ));
                                Some(delims)
                            });
                        eco_vec![func_literal_error(node, name, delims, overall_span)]
                    }
                }
            }))
        });
        Ok(Content::sequence(iter.collect::<SourceResult<Vec<_>>>()?))
    }
}

impl Eval for ast::MathText<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        match self.get() {
            MathTextKind::Grapheme(text) => Ok(SymbolElem::packed(text.clone())),
            MathTextKind::Number(text) => Ok(TextElem::packed(text.clone())),
        }
    }
}

impl Eval for ast::MathIdent<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        Ok(vm
            .scopes
            .get_in_math(&self)
            .at(span)?
            .read_checked((&mut vm.engine, span))
            .clone())
    }
}

impl Eval for ast::MathFieldAccess<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let target = self.target().eval(vm)?;
        let field = self.field();
        crate::code::access_field(vm, target, field.as_str(), field.span())
    }
}

impl Eval for ast::MathAccess<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = match self {
            ast::MathAccess::MathIdent(ident) => ident.eval(vm)?,
            ast::MathAccess::MathFieldAccess(access) => access.eval(vm)?,
        };
        // We need to call `trace_at` for the value because we did not evaluate
        // via `ast::Expr::eval()`.
        vm.trace_at(self.span(), &value);
        Ok(value)
    }
}

impl Eval for ast::MathShorthand<'_> {
    type Output = Value;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Symbol(Symbol::runtime_char(self.get())))
    }
}

impl Eval for ast::MathAlignPoint<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(AlignPointElem::shared().clone())
    }
}

impl Eval for ast::MathDelimited<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let open = self.open().eval_display(vm)?;
        let body = self.body().eval(vm)?;
        let close = self.close().eval_display(vm)?;
        Ok(LrElem::new(open + body + close).pack())
    }
}

impl Eval for ast::MathAttach<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let base = self.base().eval_display(vm)?;
        let mut elem = AttachElem::new(base);

        if let Some(expr) = self.top() {
            elem.t.set(Some(expr.eval_display(vm)?));
        }

        // Always attach primes in scripts style (not limits style),
        // i.e. at the top-right corner.
        if let Some(primes) = self.primes() {
            elem.tr.set(Some(primes.eval(vm)?));
        }

        if let Some(expr) = self.bottom() {
            elem.b.set(Some(expr.eval_display(vm)?));
        }

        Ok(elem.pack())
    }
}

impl Eval for ast::MathPrimes<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(PrimesElem::new(self.count()).pack())
    }
}

impl Eval for ast::MathFrac<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let num_expr = self.num();
        let num = num_expr.eval_display(vm)?;
        let denom_expr = self.denom();
        let denom = denom_expr.eval_display(vm)?;

        let num_depar =
            matches!(num_expr, ast::Expr::Math(math) if math.was_deparenthesized());
        let denom_depar =
            matches!(denom_expr, ast::Expr::Math(math) if math.was_deparenthesized());

        Ok(FracElem::new(num, denom)
            .with_num_deparenthesized(num_depar)
            .with_denom_deparenthesized(denom_depar)
            .pack())
    }
}

impl Eval for ast::MathRoot<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        // Use `TextElem` to match `MathTextKind::Number` above.
        let index = self.index().map(|i| TextElem::packed(eco_format!("{i}")));
        let radicand = self.radicand().eval_display(vm)?;
        Ok(RootElem::new(radicand).with_index(index).pack())
    }
}

trait ExprExt<'a> {
    fn eval_display(self, vm: &mut Vm) -> Result<Content, MathError<'a>>;
}

impl<'a> ExprExt<'a> for ast::Expr<'a> {
    /// Evaluate the expression as content for math.
    fn eval_display(self, vm: &mut Vm) -> Result<Content, MathError<'a>> {
        let value = self.eval(vm)?;
        // Symbols can cast to functions, but we don't error since they're also
        // valid as content.
        if !matches!(value, Value::Symbol(_))
            && let Ok(func_value) = value.clone().cast::<Func>()
        {
            return Err(MathError::FuncLiteral {
                node: self.to_untyped(),
                name: func_value.name().map(|name| name.into()),
            });
        }
        Ok(value.display().spanned(self.span()))
    }
}

/// An error wrapper that allows adding custom hints for function literals
/// displayed in math.
pub(crate) enum MathError<'a> {
    /// A normal source error.
    Normal(EcoVec<SourceDiagnostic>),
    /// An attempt to display a function literal in math.
    FuncLiteral { node: &'a SyntaxNode, name: Option<EcoString> },
}

impl From<EcoVec<SourceDiagnostic>> for MathError<'_> {
    fn from(value: EcoVec<SourceDiagnostic>) -> Self {
        Self::Normal(value)
    }
}

impl From<MathError<'_>> for EcoVec<SourceDiagnostic> {
    fn from(value: MathError) -> Self {
        match value {
            MathError::Normal(err) => err,
            MathError::FuncLiteral { node, name } => {
                eco_vec![func_literal_error(node, name, None, None)]
            }
        }
    }
}

/// Error for a function literal in math, potentially with hints for following
/// delimiters.
#[cold]
fn func_literal_error(
    node: &SyntaxNode,
    name: Option<EcoString>,
    delims: Option<ast::MathDelimited>,
    overall_span: Option<DiagSpan>,
) -> SourceDiagnostic {
    let func;
    let mut error;
    match node.kind() {
        // Identifier-like kinds that are reasonable to give custom hints.
        // Normal field access isn't worth handling.
        SyntaxKind::Ident | SyntaxKind::MathIdent | SyntaxKind::MathFieldAccess => {
            func = node.full_text();
            let span = overall_span.unwrap_or(node.span().into());
            error = error!(span, "this does not call the `{func}` function");
        }
        kind => {
            error = error!(node.span(), "expected content, found function");
            if let Some(name) = name {
                error.hint(eco_format!("evaluated to the `{name}` function"));
            }
            if kind == SyntaxKind::MathCall {
                // `MathCall` is the only kind that can produce a function
                // literal but cannot be called by adding trailing parentheses
                // (writing `$func()()$` doesn't work), so we just return
                // without adding extra hints.
                return error;
            }
            func = node.full_text();
        }
    }

    match delims {
        None => error.hint(eco_format!(
            "to call the function, specify arguments in parentheses: `{func}()`"
        )),
        Some(delims) => {
            if let ast::Expr::MathText(open) = delims.open()
                && let ast::Expr::MathText(close) = delims.close()
                && open.to_untyped().leaf_text() == "("
                && close.to_untyped().leaf_text() == ")"
            {
                error.hint(eco_format!(
                    "to call the function, write `{func}{}`",
                    delims.to_untyped().full_text()
                ));
                error.spanned_hint(
                    "the parentheses must directly follow the function",
                    delims.span(),
                );
            } else {
                error.hint(eco_format!(
                    "to call the function, write `{func}({})`",
                    delims.body().to_untyped().full_text()
                ));
                error.spanned_hint(
                    "functions can only be called with matched parentheses",
                    delims.span(),
                );
            }
        }
    }

    error
}

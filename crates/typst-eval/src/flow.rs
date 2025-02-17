use typst_library::diag::{bail, error, At, SourceDiagnostic, SourceResult};
use typst_library::foundations::{ops, IntoValue, Value};
use typst_syntax::ast::{self, AstNode};
use typst_syntax::{Span, SyntaxKind, SyntaxNode};
use unicode_segmentation::UnicodeSegmentation;

use crate::{destructure, Eval, Vm};

/// The maximum number of loop iterations.
const MAX_ITERATIONS: usize = 10_000;

/// A control flow event that occurred during evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum FlowEvent {
    /// Stop iteration in a loop.
    Break(Span),
    /// Skip the remainder of the current iteration in a loop.
    Continue(Span),
    /// Stop execution of a function early, optionally returning an explicit
    /// value. The final boolean indicates whether the return was conditional.
    Return(Span, Option<Value>, bool),
}

impl FlowEvent {
    /// Return an error stating that this control flow is forbidden.
    pub fn forbidden(&self) -> SourceDiagnostic {
        match *self {
            Self::Break(span) => {
                error!(span, "cannot break outside of loop")
            }
            Self::Continue(span) => {
                error!(span, "cannot continue outside of loop")
            }
            Self::Return(span, _, _) => {
                error!(span, "cannot return outside of function")
            }
        }
    }
}

impl Eval for ast::Conditional<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let condition = self.condition();
        let output = if condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            self.if_body().eval(vm)?
        } else if let Some(else_body) = self.else_body() {
            else_body.eval(vm)?
        } else {
            Value::None
        };

        // Mark the return as conditional.
        if let Some(FlowEvent::Return(_, _, conditional)) = &mut vm.flow {
            *conditional = true;
        }

        Ok(output)
    }
}

impl Eval for ast::WhileLoop<'_> {
    type Output = Value;

    #[typst_macros::time(name = "while loop", span = self.span())]
    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let flow = vm.flow.take();
        let mut output = Value::None;
        let mut i = 0;

        let condition = self.condition();
        let body = self.body();

        while condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            if i == 0
                && is_invariant(condition.to_untyped())
                && !can_diverge(body.to_untyped())
            {
                bail!(condition.span(), "condition is always true");
            } else if i >= MAX_ITERATIONS {
                bail!(self.span(), "loop seems to be infinite");
            }

            let value = body.eval(vm)?;
            let span = body.span();
            output = ops::join(output, value, &mut (&mut vm.engine, span)).at(span)?;

            match vm.flow {
                Some(FlowEvent::Break(_)) => {
                    vm.flow = None;
                    break;
                }
                Some(FlowEvent::Continue(_)) => vm.flow = None,
                Some(FlowEvent::Return(..)) => break,
                None => {}
            }

            i += 1;
        }

        if flow.is_some() {
            vm.flow = flow;
        }

        // Mark the return as conditional.
        if let Some(FlowEvent::Return(_, _, conditional)) = &mut vm.flow {
            *conditional = true;
        }

        Ok(output)
    }
}

impl Eval for ast::ForLoop<'_> {
    type Output = Value;

    #[typst_macros::time(name = "for loop", span = self.span())]
    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let flow = vm.flow.take();
        let mut output = Value::None;

        macro_rules! iter {
            (for $pat:ident in $iterable:expr) => {{
                vm.scopes.enter();

                #[allow(unused_parens)]
                for value in $iterable {
                    destructure(vm, $pat, value.into_value())?;

                    let body = self.body();
                    let value = body.eval(vm)?;
                    let span = body.span();
                    output =
                        ops::join(output, value, &mut (&mut vm.engine, span)).at(span)?;

                    match vm.flow {
                        Some(FlowEvent::Break(_)) => {
                            vm.flow = None;
                            break;
                        }
                        Some(FlowEvent::Continue(_)) => vm.flow = None,
                        Some(FlowEvent::Return(..)) => break,
                        None => {}
                    }
                }

                vm.scopes.exit();
            }};
        }

        let pattern = self.pattern();
        let iterable = self.iterable().eval(vm)?;
        let iterable_type = iterable.ty();

        use ast::Pattern;
        match (pattern, iterable) {
            (_, Value::Array(array)) => {
                // Iterate over values of array.
                iter!(for pattern in array);
            }
            (_, Value::Dict(dict)) => {
                // Iterate over key-value pairs of dict.
                iter!(for pattern in dict.iter());
            }
            (Pattern::Normal(_) | Pattern::Placeholder(_), Value::Str(str)) => {
                // Iterate over graphemes of string.
                iter!(for pattern in str.as_str().graphemes(true));
            }
            (Pattern::Normal(_) | Pattern::Placeholder(_), Value::Bytes(bytes)) => {
                // Iterate over the integers of bytes.
                iter!(for pattern in bytes.as_slice());
            }
            (Pattern::Destructuring(_), Value::Str(_) | Value::Bytes(_)) => {
                bail!(pattern.span(), "cannot destructure values of {}", iterable_type);
            }
            _ => {
                bail!(self.iterable().span(), "cannot loop over {}", iterable_type);
            }
        }

        if flow.is_some() {
            vm.flow = flow;
        }

        // Mark the return as conditional.
        if let Some(FlowEvent::Return(_, _, conditional)) = &mut vm.flow {
            *conditional = true;
        }

        Ok(output)
    }
}

impl Eval for ast::LoopBreak<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.flow.is_none() {
            vm.flow = Some(FlowEvent::Break(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ast::LoopContinue<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.flow.is_none() {
            vm.flow = Some(FlowEvent::Continue(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ast::FuncReturn<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.body().map(|body| body.eval(vm)).transpose()?;
        if vm.flow.is_none() {
            vm.flow = Some(FlowEvent::Return(self.span(), value, false));
        }
        Ok(Value::None)
    }
}

/// Whether the expression always evaluates to the same value.
fn is_invariant(expr: &SyntaxNode) -> bool {
    match expr.cast() {
        Some(ast::Expr::Ident(_)) => false,
        Some(ast::Expr::MathIdent(_)) => false,
        Some(ast::Expr::FieldAccess(access)) => {
            is_invariant(access.target().to_untyped())
        }
        Some(ast::Expr::FuncCall(call)) => {
            is_invariant(call.callee().to_untyped())
                && is_invariant(call.args().to_untyped())
        }
        _ => expr.children().all(is_invariant),
    }
}

/// Whether the expression contains a break or return.
fn can_diverge(expr: &SyntaxNode) -> bool {
    matches!(expr.kind(), SyntaxKind::Break | SyntaxKind::Return)
        || expr.children().any(can_diverge)
}

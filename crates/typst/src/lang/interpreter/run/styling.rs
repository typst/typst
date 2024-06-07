use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    Args, ContextElem, NativeElement, Recipe, ShowableSelector, Transformation, Value,
};
use crate::lang::interpreter::Vm;
use crate::lang::opcodes::{Contextual, Set, Show, ShowSet};
use crate::lang::operands::Readable;

use super::SimpleRun;

impl SimpleRun for Set {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Load the arguments.
        let args = match self.args {
            Readable::Reg(reg) => vm.take(reg).into_owned(),
            other => vm.read(other).clone(),
        };

        let args = match args {
            Value::None => Args::new::<Value>(span, []),
            Value::Args(args) => args,
            _ => {
                bail!(
                    span,
                    "expected arguments or none, found {}",
                    args.ty().long_name()
                );
            }
        };

        // Load the target function.
        let target = match vm.read(self.target) {
            Value::Func(func) => {
                if let Some(elem) = func.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            Value::Type(ty) => {
                if let Some(elem) = ty.constructor().at(span)?.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            other => bail!(span, "expected function, found {}", other.ty()),
        };

        // Build the rule and apply it.
        let set_rule = target.set(engine, args)?.spanned(span);
        vm.styled(set_rule).at(span)?;

        Ok(())
    }
}
impl SimpleRun for Show {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Load the selector.
        let selector = self
            .selector
            .map(|selector| vm.read(selector).clone().cast::<ShowableSelector>())
            .transpose()
            .at_with(|| vm.read(self.selector_span))?;

        // Load the transform.
        let transform =
            vm.read(self.transform).clone().cast::<Transformation>().at(span)?;

        // Create the show rule.
        let show_rule = Recipe {
            span,
            selector: selector.map(|selector| selector.0),
            transform,
        };

        // Write the value to the output.
        vm.recipe(show_rule).at(span)?;

        Ok(())
    }
}

impl SimpleRun for ShowSet {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Load the selector.
        let selector = self
            .selector
            .map(|selector| vm.read(selector).clone().cast::<ShowableSelector>())
            .transpose()
            .at_with(|| vm.read(self.selector_span))?;

        // Load the arguments.
        let args = match self.args {
            Readable::Reg(reg) => vm.take(reg).into_owned(),
            other => vm.read(other).clone(),
        };

        let args = match args {
            Value::None => Args::new::<Value>(span, []),
            Value::Args(args) => args,
            _ => {
                bail!(
                    span,
                    "expected arguments or none, found {}",
                    args.ty().long_name()
                );
            }
        };

        // Load the target function.
        let target = match vm.read(self.target) {
            Value::Func(func) => {
                if let Some(elem) = func.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            Value::Type(ty) => {
                if let Some(elem) = ty.constructor().at(span)?.element() {
                    elem
                } else {
                    bail!(span, "only element functions can be used in set rules")
                }
            }
            other => {
                bail!(span, "expected function, found {}", other.ty())
            }
        };

        // Create the show rule.
        let set_rule = target.set(engine, args)?.spanned(span);
        let show_rule = Recipe {
            span,
            selector: selector.map(|selector| selector.0),
            transform: Transformation::Style(set_rule),
        };

        // Write the value to the output.
        vm.recipe(show_rule).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Contextual {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Load the context.
        let closure = vm.read(self.closure);

        // Load the value.
        let Value::Func(closure) = closure else {
            bail!(span, "expected closure, found {}", closure.ty());
        };

        // Write the value to the output.
        vm.write_one(self.out, ContextElem::new(closure.clone()).pack().spanned(span))
            .at(span)?;

        Ok(())
    }
}

use std::borrow::Cow;

use typst_syntax::{Span, Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, At, SourceResult, Trace, Tracepoint};
use crate::engine::{Engine, Route};
use crate::foundations::{
    call_method_mut, Arg, Args, Array, Bytes, Content, ContextElem, Dict, Func,
    IntoValue, NativeElement, Recipe, SequenceElem, ShowableSelector, Smart,
    Transformation, Value,
};
use crate::lang::compiled::CompiledAccess;
use crate::lang::compiler::{import_value, ImportedModule};
use crate::lang::interpreter::ControlFlow;
use crate::lang::operands::Register;
use crate::lang::{opcodes::*, ops};
use crate::math::{
    Accent, AccentElem, AttachElem, EquationElem, FracElem, LrElem, RootElem,
};
use crate::model::{EmphElem, HeadingElem, RefElem, StrongElem, Supplement};
use crate::symbols::Symbol;
use crate::text::TextElem;

use super::{ValueAccessor, Vm};

pub trait Run {
    /// Runs an opcode.
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        iterator: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()>;
}

pub trait SimpleRun {
    /// Runs an opcode.
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()>;
}

impl<T: SimpleRun> Run for T {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        <T as SimpleRun>::run(self, span, vm, engine)
    }
}

impl SimpleRun for Add {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Add the right-hand side from the left-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::add(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Sub {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Subtract the right-hand side from the left-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::sub(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Mul {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Multiply the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::mul(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Div {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Divide the left-hand side by the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::div(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Neg {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::neg(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Pos {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Positive the value and write the result to the output.
        vm.write_one(self.out, ops::pos(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Not {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Negate the value and write the result to the output.
        vm.write_one(self.out, ops::not(value).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Gt {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::gt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Geq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::geq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Lt {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::lt(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Leq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::leq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Eq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::eq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Neq {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Compare the left-hand side to the right-hand side and write the
        // result to the output.
        vm.write_one(self.out, ops::neq(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for In {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and the iterator.
        let value = vm.read(self.lhs);
        let iterator = vm.read(self.rhs);

        // Check if the value is in the iterator and write the result to the
        // output.
        vm.write_one(self.out, ops::in_(value, iterator).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for NotIn {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and the iterator.
        let value = vm.read(self.lhs);
        let iterator = vm.read(self.rhs);

        // Check if the value is not in the iterator and write the result to the
        // output.
        vm.write_one(self.out, ops::not_in(value, iterator).at(span)?)
            .at(span)?;

        Ok(())
    }
}

impl SimpleRun for And {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // And the left-hand side with the right-hand side and write the result
        // to the output.
        vm.write_one(self.out, ops::and(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Or {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left-hand side and right-hand side.
        let lhs = vm.read(self.lhs);
        let rhs = vm.read(self.rhs);

        // Or the left-hand side with the right-hand side and write the result
        // to the output.
        vm.write_one(self.out, ops::or(lhs, rhs).at(span)?).at(span)?;

        Ok(())
    }
}

impl SimpleRun for CopyIsr {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl SimpleRun for ReadAccess {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Get the access.
        let access = vm.read(self.access);

        // Get the value.
        let value = access.read(span, vm)?.into_owned();

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl SimpleRun for None {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Write `None` to the output.
        vm.write_one(self.out, Value::None).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Auto {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Write the value to the output.
        vm.write_one(self.out, Value::Auto).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Assign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        assign(span, vm, engine, self.value, self.out)
    }
}

impl SimpleRun for AddAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::add(&old, &value).at(span)
        })
    }
}

impl SimpleRun for SubAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::sub(&old, &value).at(span)
        })
    }
}

impl SimpleRun for MulAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::mul(&old, &value).at(span)
        })
    }
}

impl SimpleRun for DivAssign {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        let lhs_span = vm.read(self.lhs_span);
        assign_op(lhs_span, vm, engine, self.value, self.out, |old, value| {
            ops::div(&old, &value).at(span)
        })
    }
}

impl SimpleRun for Destructure {
    fn run(&self, _: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.value).clone();

        // Get the pattern.
        let pattern = vm.read(self.out);

        // Destructure the value.
        pattern.write(vm, engine, value)?;

        Ok(())
    }
}

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

impl SimpleRun for InstantiateModule {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Load the path to the module.
        let path = vm.read(self.path);

        // Load the module description
        let module = vm.read(self.module);

        // Load the module, we know it's static.
        let ImportedModule::Static(loaded) = import_value(engine, path, span, true)? else {
            bail!(span, "expected static module, found dynamic module");
        };

        // Iterate over the module description and apply the rules.
        for value in &module.imports {
            let field = loaded.field(value.name).at(value.span)?;

            // Apply the rule.
            vm.write_one(value.location, field.clone()).at(value.span)?;
        }

        // Write the module to the output.
        vm.write_one(self.out, Value::Module(loaded)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Include {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Load the path to the module.
        let path = vm.read(self.path);

        // Load the module, we know it's static.
        let ImportedModule::Static(loaded) = import_value(engine, path, span, false)? else {
            bail!(span, "expected static module, found dynamic module");
        };

        // Write the module's content to the output.
        vm.write_one(self.out, loaded.content()).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Instantiate {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Get the closure.
        let closure = vm.read(self.closure);
        let closure_span = closure.span();

        // Instantiate the closure. This involves:
        // - Capturing all necessary values.
        // - Capturing the default values of named arguments.
        let closure = vm.instantiate(closure)?;

        // Write the closure to the output.
        vm.write_one(self.out, Func::from(closure.into_owned()).spanned(closure_span))
            .at(span)?;

        Ok(())
    }
}

impl SimpleRun for Call {
    fn run(
        &self,
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        // Check that we're not exceeding the call depth limit.
        if !engine.route.within(Route::MAX_CALL_DEPTH) {
            bail!(span, "maximum function call depth exceeded");
        }

        // Get the function.
        let accessor = vm.read(self.closure);

        // Get the arguments.
        let args = match self.args {
            Readable::Reg(reg) => vm.take(reg).into_owned(),
            other => vm.read(other).clone(),
        };

        let mut args = match args {
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

        let callee_span = vm.read(self.callee_span);

        // First we read the value and we check whether this is a mutable call.
        if let CompiledAccess::Chained(_, rest, method, _) = accessor {
            // Obtain the value.
            let access = vm.read(*rest);
            let value = access.read(callee_span, vm)?;

            // Check whether the method is mutable or not.
            // If it is mut:
            //  - Redo the access as mutable
            //  - Call the function.
            if value.is_mut(method) {
                let mut value = access.write(callee_span, vm, engine)?;

                let point = || Tracepoint::Call(Some((*method).into()));
                let value = call_method_mut(&mut value, *method, args, span)
                    .trace(engine.world, point, span)?
                    .spanned(span);

                // Write the value to the output.
                vm.write_one(self.out, value).at(span)?;

                return Ok(());
            }
        }

        // If it is not a mutable call, we proceed as usual:
        // - We check for methods.
        // - We read the accessor
        // - We check for math-specific cases
        // - We call the method
        let callee =
            if let CompiledAccess::Chained(_, rest, method, field_span) = accessor {
                // Obtain the value.
                let access = vm.read(*rest);
                let value = access.read(callee_span, vm)?;

                // Check if we are calling a method.
                if let Some(callee) = value.ty().scope().get(*method) {
                    let this = Arg {
                        span,
                        name: None,
                        value: Spanned::new(value.into_owned(), *field_span),
                    };
                    args.items.insert(0, this);

                    Cow::Borrowed(callee)
                } else if let Value::Plugin(plugin) = &*value {
                    let bytes = args.all::<Bytes>()?;
                    args.finish()?;

                    let out = plugin.call(*method, bytes).at(span)?.into_value();

                    // Write the value to the output.
                    vm.write_one(self.out, out).at(span)?;

                    return Ok(());
                } else {
                    accessor.read(callee_span, vm)?
                }
            } else {
                accessor.read(callee_span, vm)?
            };

        // Special case handling for equations.
        if self.math && !matches!(&*callee, Value::Func(_)) {
            if let Value::Symbol(sym) = &*callee {
                let c = sym.get();
                if let Some(accent) = Symbol::combining_accent(c) {
                    let base = args.expect("base")?;
                    let size = args.named("size")?;
                    args.finish()?;
                    let mut accent = AccentElem::new(base, Accent::new(accent));
                    if let Some(size) = size {
                        accent = accent.with_size(size);
                    }

                    // Write the value to the output.
                    vm.write_one(self.out, accent.pack().spanned(span)).at(span)?;

                    return Ok(());
                }
            }

            let mut body = Content::empty();
            for (i, arg) in args.all::<Content>()?.into_iter().enumerate() {
                if i > 0 {
                    body += TextElem::packed(',');
                }
                body += arg;
            }

            if self.trailing_comma {
                body += TextElem::packed(',');
            }

            let out = callee.into_owned().display().spanned(span)
                + LrElem::new(TextElem::packed('(') + body + TextElem::packed(')'))
                    .pack()
                    .spanned(span);

            // Write the value to the output.
            vm.write_one(self.out, out).at(span)?;

            return Ok(());
        }

        // Call the function
        let point = || Tracepoint::Call(callee.name().map(Into::into));
        let value = callee
            .call(engine, vm.context, span, args)
            .trace(engine.world, point, span)?
            .spanned(span);

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Field {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Get the value.
        let value = vm.read(self.access).read(span, vm)?;

        // Write the value to the output.
        // TODO: improve efficiency by removing cloning!
        vm.write_one(self.out, value.into_owned()).at(span)?;

        Ok(())
    }
}

impl Run for While {
    #[typst_macros::time(name = "while loop", span = span)]
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let instructions = &instructions[..self.len as usize];
        let spans = &spans[..self.len as usize];

        // Runt the loop inside a new scope.
        let flow = vm.enter_scope(
            engine,
            instructions,
            spans,
            None,
            None,
            self.content,
            true,
        )?;

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(_) | ControlFlow::Continue(_) => {
                bail!(span, "unexpected control flow, malformed instruction")
            }
            ControlFlow::Return(value, forced) => {
                vm.state.set_returning(forced);
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else {
            // Write the output to the output register.
            vm.write_one(self.out, output).at(span)?;
        }

        vm.bump(self.len as usize);

        Ok(())
    }
}

impl Run for Iter {
    #[typst_macros::time(name = "for loop", span = span)]
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        // Get the iterable.
        let iterable = vm.read(self.iterable).clone();
        let instructions = &instructions[..self.len as usize];

        macro_rules! iter {
            (for $iterable:expr) => {{
                let mut iter = $iterable.into_iter().map(IntoValue::into_value);
                vm.enter_scope(
                    engine,
                    instructions,
                    spans,
                    Some(&mut iter),
                    None,
                    self.content,
                    true,
                )?
            }};
        }

        let iterable_type = iterable.ty();
        let flow = match iterable {
            Value::Array(array) => {
                // Iterate over values of array.
                iter!(for array)
            }
            Value::Dict(dict) => {
                // Iterate over key-value pairs of dict.
                iter!(for dict.iter())
            }
            Value::Str(str) => {
                // Iterate over graphemes of string.
                iter!(for str.as_str().graphemes(true))
            }
            Value::Bytes(bytes) => {
                // Iterate over the integers of bytes.
                iter!(for bytes.as_slice().into_iter().map(|byte| Value::Int(*byte as i64)))
            }
            _ => {
                bail!(span, "cannot loop over {}", iterable_type);
            }
        };

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(_) | ControlFlow::Continue(_) => {
                unreachable!("unexpected control flow")
            }
            ControlFlow::Return(value, forced) => {
                vm.state.set_returning(forced);
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else {
            // Write the output to the output register.
            vm.write_one(self.out, output).at(span)?;
        }

        vm.bump(self.len as usize);

        Ok(())
    }
}

impl Run for Next {
    fn run(
        &self,
        _: &[Opcode],
        _: &[Span],
        span: Span,
        vm: &mut Vm,
        _: &mut Engine,
        iterator: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let Some(iter) = iterator else {
            bail!(span, "not in an iterable scope");
        };

        // Get the next value.
        let Some(value) = iter.next() else {
            vm.state.set_done();
            return Ok(());
        };

        // Write the value to the output.
        vm.write_one(self.out, value).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Continue {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        if !vm.state.is_breaking() && !vm.state.is_returning() {
            vm.state.set_continuing();
        }

        Ok(())
    }
}

impl SimpleRun for Break {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        if !vm.state.is_continuing() && !vm.state.is_returning() {
            vm.state.set_breaking();
        }

        Ok(())
    }
}

impl SimpleRun for Return {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        if !vm.state.is_breaking() && !vm.state.is_continuing() {
            vm.state.set_returning(vm.output.is_some());
        }

        Ok(())
    }
}

impl SimpleRun for ReturnVal {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        vm.output = Some(self.value.into());
        if !vm.state.is_breaking() && !vm.state.is_continuing() {
            vm.state.set_returning(vm.output.is_some());
        }

        Ok(())
    }
}

impl SimpleRun for AllocArray {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Create a new array.
        let array = Value::Array(Array::with_capacity(self.capacity as usize));

        // Write the array to the output.
        vm.write_one(self.out, array).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Push {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the array.
        let Some(Value::Array(array)) = vm.write(self.out) else {
            bail!(span, "expected array, found {}", value.ty().long_name());
        };

        array.push(value);

        Ok(())
    }
}

impl SimpleRun for AllocDict {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Create a new dictionary.
        let dict = Value::Dict(Dict::with_capacity(self.capacity as usize));

        // Write the dictionary to the output.
        vm.write_one(self.out, dict).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Insert {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Obtain the key.
        let Value::Str(key) = vm.read(self.key).clone() else {
            bail!(span, "expected string, found {}", value.ty().long_name());
        };

        // Get a mutable reference to the dictionary.
        let Some(Value::Dict(dict)) = vm.write(self.out) else {
            bail!(span, "expected dictionary, found {}", value.ty().long_name());
        };

        dict.insert(key, value);

        Ok(())
    }
}

impl SimpleRun for AllocArgs {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Create a new argument set.
        let args = Value::Args(Args::with_capacity(span, self.capacity as usize));

        // Write the argument set to the output.
        vm.write_one(self.out, args).at(span)?;

        Ok(())
    }
}

impl SimpleRun for PushArg {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the argument set.
        let Some(Value::Args(args)) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty().long_name());
        };

        args.push(span, value_span, value);

        Ok(())
    }
}

impl SimpleRun for InsertArg {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get the argument name.
        let Value::Str(name) = vm.read(self.key).clone() else {
            bail!(span, "expected string, found {}", value.ty());
        };

        // Get a mutable reference to the argument set.
        let Some(Value::Args(args)) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty());
        };

        args.insert(span, value_span, name, value);

        Ok(())
    }
}

impl SimpleRun for SpreadArg {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value's span.
        let value_span = vm.read(self.value_span);

        // Obtain the value.
        let value = vm.read(self.value).clone();

        // Get a mutable reference to the argument set.
        let Some(Value::Args(into)) = vm.write(self.out) else {
            bail!(span, "expected argument set, found {}", value.ty().long_name());
        };

        match value {
            Value::Args(args_) => {
                into.chain(args_);
            }
            Value::Dict(dict) => {
                into.extend(dict.into_iter().map(|(name, value)| Arg {
                    span,
                    name: Some(name),
                    value: Spanned::new(value, value_span),
                }));
            }
            Value::Array(array) => {
                into.extend(array.into_iter().map(|value| Arg {
                    span,
                    name: None,
                    value: Spanned::new(value, value_span),
                }));
            }
            Value::None => {}
            _ => {
                bail!(span, "cannot spread {}", value.ty());
            }
        }

        Ok(())
    }
}

impl SimpleRun for Spread {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value).clone();

        match vm.write(self.out) {
            Some(Value::Array(into)) => match value {
                Value::Array(array) => {
                    into.extend(array.into_iter());
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {} into array", value.ty());
                }
            },
            Some(Value::Dict(into)) => match value {
                Value::Dict(dict) => {
                    into.extend(dict.into_iter());
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {} into dictionary", value.ty());
                }
            },
            Some(Value::Args(into)) => match value {
                Value::Args(args_) => {
                    into.chain(args_);
                }
                Value::Dict(dict) => {
                    into.extend(dict.into_iter().map(|(name, value)| Arg {
                        span,
                        name: Some(name),
                        value: Spanned::new(value, span),
                    }));
                }
                Value::Array(array) => {
                    into.extend(array.into_iter().map(|value| Arg {
                        span,
                        name: None,
                        value: Spanned::new(value, span),
                    }));
                }
                Value::None => {}
                _ => {
                    bail!(span, "cannot spread {} into arguments", value.ty());
                }
            },
            _ => {
                bail!(
                    span,
                    "expected array, dictionary, or arguments, found {}",
                    value.ty().long_name()
                );
            }
        }

        Ok(())
    }
}

impl Run for Enter {
    fn run(
        &self,
        instructions: &[Opcode],
        spans: &[Span],
        span: Span,
        vm: &mut Vm,
        engine: &mut Engine,
        _: Option<&mut dyn Iterator<Item = Value>>,
    ) -> SourceResult<()> {
        let instructions = &instructions[..self.len as usize];
        let spans = &spans[..self.len as usize];

        // Enter the scope within the vm.
        let flow = vm.enter_scope(
            engine,
            instructions,
            spans,
            None,
            None,
            self.content,
            false,
        )?;

        let mut forced_return = false;
        let output = match flow {
            ControlFlow::Done(value) => value,
            ControlFlow::Break(value) => {
                vm.state.set_breaking();
                value
            }
            ControlFlow::Continue(value) => {
                vm.state.set_continuing();
                value
            }
            ControlFlow::Return(value, forced) => {
                vm.state.set_returning(forced);
                forced_return = forced;
                value
            }
        };

        if forced_return {
            let reg = Register(0);
            vm.write_one(reg, output).at(span)?;
            vm.output = Some(Readable::reg(reg));
        } else {
            // Write the output to the output register.
            vm.write_one(self.out, output).at(span)?;
        }

        vm.bump(self.len as usize);

        Ok(())
    }
}

impl SimpleRun for PointerMarker {
    fn run(&self, _: Span, _: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        Ok(())
    }
}

impl SimpleRun for JumpTop {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        vm.jump(0);
        Ok(())
    }
}

impl SimpleRun for Jump {
    fn run(&self, _: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Jump to the instruction.
        vm.jump(vm.read(self.instruction));

        Ok(())
    }
}

impl SimpleRun for JumpIf {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is true.
        if *condition {
            vm.jump(vm.read(self.instruction));
        }

        Ok(())
    }
}

impl SimpleRun for JumpIfNot {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Jump to the instruction if the condition is true.
        if !*condition {
            vm.jump(vm.read(self.instruction));
        }

        Ok(())
    }
}

impl SimpleRun for BeginIter {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        if vm.iter() > 100_000 {
            bail!(span, "loop seems to be infinite");
        }

        Ok(())
    }
}

impl SimpleRun for Select {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the condition.
        let condition = vm.read(self.condition);

        // Get the condition as a boolean.
        let Value::Bool(condition) = condition else {
            bail!(span, "expected boolean, found {}", condition.ty().long_name());
        };

        // Select the true value if the condition is true, otherwise select the
        // false value.
        let value = if *condition { vm.read(self.true_) } else { vm.read(self.false_) };

        // Write the value to the output.
        vm.write_one(self.out, value.clone()).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Delimited {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left delimiter, body, and right delimiter.
        let left: Content = vm.read(self.left).clone().display();
        let body: Content = vm.read(self.body).clone().display();
        let right: Content = vm.read(self.right).clone().display();

        // Make the value into a delimited.
        let value = LrElem::new(
            SequenceElem::new(vec![left.into(), body.into(), right.into()])
                .pack()
                .spanned(span),
        );

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Attach {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the base, top, and bottom.
        let base = vm.read(self.base);
        let top = vm.read(self.top);
        let bottom = vm.read(self.bottom);

        // Make the value into an attach.
        let mut value = AttachElem::new(base.clone().display());

        if let Some(top) = top {
            value.push_t(Some(top.clone().display()));
        }

        if let Some(bottom) = bottom {
            value.push_b(Some(bottom.clone().display()));
        }

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Frac {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the numerator and denominator.
        let numerator = vm.read(self.numerator);
        let denominator = vm.read(self.denominator);

        // Make the value into a fraction.
        let value =
            FracElem::new(numerator.clone().display(), denominator.clone().display());

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Root {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the degree and radicand.
        let degree = vm.read(self.degree);
        let radicand = vm.read(self.radicand);

        // Make the value into a root.
        let mut value = RootElem::new(radicand.clone().display());

        if let Some(degree) = degree {
            value.push_index(Some(degree.clone().display()));
        }

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Ref {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the supplement.
        let supplement = vm.read(self.supplement);

        // Read the label.
        let value = vm.read(self.label);
        let Value::Label(label) = value else {
            bail!(span, "expected label, found {}", value.ty().long_name());
        };

        // Create the reference.
        let reference = RefElem::new(*label)
            .with_supplement(Smart::Custom(Some(Supplement::Content(
                supplement.clone().display(),
            ))))
            .pack()
            .spanned(span);

        // Write the reference to the output.
        vm.write_one(self.out, reference).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Strong {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value strong.
        let value = StrongElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Emph {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value emphasized.
        let value = EmphElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Heading {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and level.
        let value = vm.read(self.value);
        let level = self.level;

        // Make the value into a heading.
        let mut value = HeadingElem::new(value.clone().cast().at(span)?);

        // Set the level of the heading.
        value.push_level(Smart::Custom(level.into()));

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for ListItem {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value into a list item.
        let value = crate::model::ListItem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for EnumItem {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and number.
        let value = vm.read(self.value);
        let number = self.number.map(|number| number.get() as usize - 1);

        // Make the value into an enum item.
        let value = crate::model::EnumItem::new(value.clone().cast().at(span)?)
            .with_number(number);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for TermItem {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and description.
        let value = vm.read(self.term);
        let description = vm.read(self.description);

        // Make the value into a term.
        let value = crate::model::TermItem::new(
            value.clone().cast().at(span)?,
            description.clone().cast().at(span)?,
        );

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Equation {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value into an equation.
        let value =
            EquationElem::new(value.clone().cast().at(span)?).with_block(self.block);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

fn assign(
    span: Span,
    vm: &mut Vm,
    engine: &mut Engine,
    value: Readable,
    out: AccessId,
) -> SourceResult<()> {
    // Get the value.
    let value = vm.read(value).clone();

    // Get the accessor.
    let access = vm.read(out);

    // Get the mutable reference to the target.
    if let CompiledAccess::Chained(_, dict, field, _) = access {
        let dict = vm.read(*dict);
        if let CompiledAccess::Register(dict) = dict {
            if let Some(Value::Dict(dict)) = vm.write(*dict) {
                dict.insert((*field).into(), value);
                return Ok(());
            }
        }
    }

    let out = access.write(span, vm, engine)?;

    // Write the value to the target.
    *out = value;

    Ok(())
}

fn assign_op(
    span: Span,
    vm: &mut Vm,
    engine: &mut Engine,
    value: Readable,
    out: AccessId,
    transformer: impl FnOnce(Value, Value) -> SourceResult<Value>,
) -> SourceResult<()> {
    // Get the value.
    let value = vm.read(value).clone();

    // Get the accessor.
    let access = vm.read(out);

    // Get the mutable reference to the target.
    if let CompiledAccess::Chained(_, dict, field, field_span) = access {
        let dict = vm.read(*dict);
        if let CompiledAccess::Register(dict) = dict {
            if let Some(Value::Dict(dict)) = vm.write(*dict) {
                let item = dict.at_mut(*field).at(*field_span)?;

                let old = std::mem::take(item);
                *item = transformer(old, value)?;

                return Ok(());
            }
        }
    }

    let out = access.write(span, vm, engine)?;

    // Write the value to the target.
    let old = std::mem::take(out);
    *out = transformer(old, value)?;

    Ok(())
}

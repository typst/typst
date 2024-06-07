use std::borrow::Cow;

use typst_syntax::{Span, Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{bail, At, SourceResult, Trace, Tracepoint};
use crate::engine::{Engine, Route};
use crate::foundations::{
    call_method_mut, Arg, Args, Bytes, Content, Func, IntoValue, NativeElement, Value,
};
use crate::lang::compiled::CompiledAccess;
use crate::lang::compiler::{import_value, ImportedModule};
use crate::lang::interpreter::methods::ValueAccessor;
use crate::lang::interpreter::{ControlFlow, Vm};
use crate::lang::opcodes::{
    Break, Call, Continue, Field, Include, Instantiate, InstantiateModule, Iter, Next,
    Opcode, Return, ReturnVal, While,
};
use crate::lang::operands::Readable;
use crate::lang::operands::Register;
use crate::math::{Accent, AccentElem, LrElem};
use crate::symbols::Symbol;
use crate::text::TextElem;

use super::{Run, SimpleRun};

impl SimpleRun for InstantiateModule {
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
        // Load the path to the module.
        let path = vm.read(self.path);

        // Load the module description
        let module = vm.read(self.module);

        // Load the module, we know it's static.
        let ImportedModule::Static(loaded) = import_value(engine, path, span, true)?
        else {
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
        let ImportedModule::Static(loaded) = import_value(engine, path, span, false)?
        else {
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
    fn run(&self, span: Span, vm: &mut Vm, engine: &mut Engine) -> SourceResult<()> {
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
        let flow =
            vm.enter_scope(engine, instructions, spans, None, None, self.content, true)?;

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

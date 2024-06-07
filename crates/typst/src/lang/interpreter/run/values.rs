use typst_syntax::{Span, Spanned};

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Arg, Args, Array, Dict, Value};
use crate::lang::interpreter::Vm;
use crate::lang::opcodes::{
    AllocArgs, AllocArray, AllocDict, Insert, InsertArg, Push, PushArg, Spread, SpreadArg,
};

use super::SimpleRun;

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

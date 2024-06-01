use std::borrow::Cow;

use typst_syntax::Span;
use typst_utils::pico;

use crate::diag::{bail, At, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{call_method_access, Args, IntoValue, Type, Value};
use crate::lang::compiled::CompiledAccess;
use crate::lang::opcodes::Readable;

use super::{Read, Vm};

impl CompiledAccess {
    /// Gets the value using read-only access.
    pub fn read<'a: 'b, 'b>(
        &'a self,
        span: Span,
        vm: &'b Vm<'a, '_>,
    ) -> SourceResult<Cow<'b, Value>> {
        match self {
            CompiledAccess::Readable(readable) => Ok(Cow::Borrowed(readable.read(vm))),
            CompiledAccess::Writable(writeable) => Ok(Cow::Borrowed(writeable.read(vm))),
            CompiledAccess::Module(module) => Ok(Cow::Borrowed(module)),
            CompiledAccess::Func(func) => Ok(Cow::Borrowed(func)),
            CompiledAccess::Value(value) => Ok(Cow::Borrowed(value)),
            CompiledAccess::Type(ty) => Ok(Cow::Borrowed(ty)),
            CompiledAccess::Chained(value, field) => {
                let access = vm.read(*value);
                let value = access.read(span, vm)?;
                if let Some(assoc) = value.ty().scope().get(field.resolve()) {
                    let Value::Func(method) = assoc else {
                        bail!(
                            span,
                            "expected function, found {}",
                            assoc.ty().long_name()
                        );
                    };

                    let mut args =
                        Args::new(Span::detached(), std::iter::once(value.into_owned()));

                    Ok(Cow::Owned(method.clone().with(&mut args).into_value()))
                } else {
                    value.field(field.resolve()).map(Cow::Owned).at(span)
                }
            }
            CompiledAccess::AccessorMethod(value, method, args) => {
                // Get the callee.
                let access = vm.read(*value);
                let value = access.read(span, vm)?;

                // Get the arguments.
                let args = vm.read(*args);
                let mut args = match args {
                    Value::Args(args) => args.clone(),
                    Value::None => Args::with_capacity(span, 0),
                    _ => bail!(
                        span,
                        "expected argumentss, found {}",
                        args.ty().long_name()
                    ),
                };

                // Call the method.
                let ty = value.ty();
                let missing = || Err(missing_method(ty, method.resolve())).at(span);

                let first = pico!("first");
                let last = pico!("last");
                let at = pico!("at");

                let accessed = match &*value {
                    Value::Array(array) => {
                        if *method == first {
                            array.first().at(span)?
                        } else if *method == last {
                            array.last().at(span)?
                        } else if *method == at {
                            array.at(args.expect("index")?, None).at(span)?
                        } else {
                            return missing();
                        }
                    }
                    Value::Dict(dict) => {
                        if *method == at {
                            dict.at(args.expect("key")?, None).at(span)?
                        } else {
                            return missing();
                        }
                    }
                    _ => return missing(),
                };

                Ok(Cow::Owned(accessed))
            }
        }
    }

    /// Gets the value using write access.
    pub fn write<'a: 'b, 'b>(
        &self,
        span: Span,
        vm: &'b mut Vm<'a, '_>,
        engine: &mut Engine,
    ) -> SourceResult<&'b mut Value> {
        match self {
            CompiledAccess::Readable(_) => {
                bail!(span, "cannot write to a readable, malformed access")
            }
            CompiledAccess::Writable(writable) => Ok(vm.write(*writable)),
            CompiledAccess::Module(_) => {
                bail!(span, "cannot write to a global, malformed access")
            }
            CompiledAccess::Func(_) => {
                bail!(span, "cannot write to a function, malformed access")
            }
            CompiledAccess::Value(_) => {
                bail!(span, "cannot write to a static value, malformed access")
            }
            CompiledAccess::Type(_) => {
                bail!(span, "cannot write to a type, malformed access")
            }
            CompiledAccess::Chained(value, field) => {
                let access = vm.read(*value);
                let value = access.write(span, vm, engine)?;
                match value {
                    Value::Dict(dict) => dict.at_mut(field.resolve()).at(span),
                    value => {
                        let ty = value.ty();
                        if matches!(
                            value, // those types have their own field getters
                            Value::Symbol(_)
                                | Value::Content(_)
                                | Value::Module(_)
                                | Value::Func(_)
                        ) {
                            bail!(span, "cannot mutate fields on {ty}");
                        } else if crate::foundations::fields_on(ty).is_empty() {
                            bail!(span, "{ty} does not have accessible fields");
                        } else {
                            // type supports static fields, which don't yet have
                            // setters
                            bail!(
                                span,
                                "fields on {ty} are not yet mutable";
                                hint: "try creating a new {ty} with the updated field value instead"
                            )
                        }
                    }
                }
            }
            CompiledAccess::AccessorMethod(value, method, args) => {
                // Get the arguments.
                let args = match *args {
                    Readable::Reg(reg) => vm.take(reg).into_owned(),
                    other => vm.read(other).clone(),
                };

                let args = match args {
                    Value::Args(args) => args.clone(),
                    Value::None => Args::with_capacity(span, 0),
                    _ => bail!(
                        span,
                        "expected argumentss, found {}",
                        args.ty().long_name()
                    ),
                };

                // Get the callee.
                let access = vm.read(*value);
                let value = access.write(span, vm, engine)?;

                let point = || Tracepoint::Call(Some(method.resolve().into()));
                call_method_access(value, method.resolve(), args, span).trace(
                    engine.world,
                    point,
                    span,
                )
            }
        }
    }
}

/// The missing method error message.
#[cold]
fn missing_method(ty: Type, method: &str) -> String {
    format!("type {ty} has no method `{method}`")
}

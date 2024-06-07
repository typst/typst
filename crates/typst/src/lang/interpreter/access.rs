use std::borrow::Cow;

use ecow::eco_vec;
use typst_syntax::Span;

use crate::diag::{bail, error, At, SourceResult, Trace, Tracepoint};
use crate::engine::Engine;
use crate::foundations::{call_method_access, Args, IntoValue, Type, Value};
use crate::lang::compiled::CompiledAccess;
use crate::lang::operands::Readable;

use super::{Read, Vm};

impl CompiledAccess {
    /// Gets the value using read-only access.
    pub fn read<'a: 'b, 'b>(
        &'a self,
        span: Span,
        vm: &'b Vm<'a, '_>,
    ) -> SourceResult<Cow<'b, Value>> {
        match self {
            CompiledAccess::Register(reg) => Ok(Cow::Borrowed(reg.read(vm))),
            CompiledAccess::Module(module) => Ok(Cow::Borrowed(module)),
            CompiledAccess::Func(func) => Ok(Cow::Borrowed(func)),
            CompiledAccess::Value(value) => Ok(Cow::Borrowed(value)),
            CompiledAccess::Type(ty) => Ok(Cow::Borrowed(ty)),
            CompiledAccess::Chained(_, value, field, field_span) => {
                let access = vm.read(*value);
                let value = access.read(span, vm)?;
                if let Some(assoc) = value.ty().scope().get(field) {
                    let Value::Func(method) = assoc else {
                        bail!(
                            span,
                            "expected function, found {}",
                            assoc.ty().long_name()
                        );
                    };

                    let mut args = Args::new(span, std::iter::once(value.into_owned()));

                    Ok(Cow::Owned(
                        method.clone().with(&mut args).into_value().spanned(span),
                    ))
                } else {
                    let err = match value.field(&field).at(*field_span) {
                        Ok(value) => return Ok(Cow::Owned(value)),
                        Err(err) => err,
                    };

                    // Check whether this is a get rule field access.
                    if_chain::if_chain! {
                        if let Value::Func(func) = &*value;
                        if let Some(element) = func.element();
                        if let Some(id) = element.field_id(&field);
                        let styles = vm.context.styles().at(*field_span);
                        if let Some(value) = element.field_from_styles(
                            id,
                            styles.as_ref().map(|&s| s).unwrap_or_default(),
                        );
                        then {
                            // Only validate the context once we know that this is indeed
                            // a field from the style chain.
                            let _ = styles?;
                            return Ok(Cow::Owned(value));
                        }
                    }

                    Err(err)
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
                let missing = || Err(missing_method(ty, method)).at(span);

                let accessed = match &*value {
                    Value::Array(array) => {
                        if *method == "first" {
                            array.first().at(span)?
                        } else if *method == "last" {
                            array.last().at(span)?
                        } else if *method == "at" {
                            array.at(args.expect("index")?, None).at(span)?
                        } else {
                            return missing();
                        }
                    }
                    Value::Dict(dict) => {
                        if *method == "at" {
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
            CompiledAccess::Register(reg) => vm.write(*reg).ok_or_else(|| {
                eco_vec![error!(span, "cannot write to a temporary value")]
            }),
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
            CompiledAccess::Chained(parent_span, value, field, field_span) => {
                let access = vm.read(*value);
                let value = access.write(span, vm, engine)?;
                match value {
                    Value::Dict(dict) => dict.at_mut(field).at(*field_span),
                    value => {
                        let ty = value.ty();
                        if matches!(
                            value, // those types have their own field getters
                            Value::Symbol(_)
                                | Value::Content(_)
                                | Value::Module(_)
                                | Value::Func(_)
                        ) {
                            bail!(*parent_span, "cannot mutate fields on {ty}");
                        } else if crate::foundations::fields_on(ty).is_empty() {
                            bail!(*parent_span, "{ty} does not have accessible fields");
                        } else {
                            // type supports static fields, which don't yet have
                            // setters
                            bail!(
                                *parent_span,
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

                let point = || Tracepoint::Call(Some((*method).into()));
                call_method_access(value, method, args, span).trace(
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

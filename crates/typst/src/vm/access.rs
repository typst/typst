use std::borrow::Cow;
use std::sync::Arc;

use ecow::EcoString;
use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::foundations::{call_method_access, Func, Str, Type, Value};

use super::{Readable, VMState, VmRead, Writable};

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum Access {
    /// Access this value through a readable.
    Readable(Readable),

    /// Access this value through a writeable.
    Writable(Writable),

    /// Access this value through a chained access.
    Chained(Arc<Self>, EcoString),

    /// Access this value through an accessor method.
    AccessorMethod(Arc<Self>, EcoString, Readable),
}

impl Access {
    /// Gets the value using read-only access.
    pub fn read<'a>(&self, span: Span, vm: &'a VMState) -> SourceResult<Cow<'a, Value>> {
        match self {
            Access::Readable(readable) => readable.read(vm).map(Cow::Borrowed).at(span),
            Access::Writable(writeable) => writeable.read(vm).map(Cow::Borrowed).at(span),
            Access::Chained(value, field) => {
                let value = value.read(span, vm)?;
                if let Some(assoc) = value.ty().scope().get(field) {
                    let Value::Func(method) = assoc else {
                        bail!(
                            span,
                            "expected function, found {}",
                            assoc.ty().long_name()
                        );
                    };

                    Ok(Cow::Owned(Value::Func(Func::method(
                        value.into_owned(),
                        method.clone(),
                    ))))
                } else {
                    value.field(field).map(Cow::Owned).at(span)
                }
            }
            Access::AccessorMethod(value, method, args) => {
                // Get the callee.
                let value = value.read(span, vm)?;

                // Get the arguments.
                let args = vm.read(*args).at(span)?;
                let Value::Args(mut args) = args.clone() else {
                    bail!(span, "expected args, found {}", args.ty().long_name());
                };

                // Call the method.
                let ty = value.ty();
                let missing = || Err(missing_method(ty, method)).at(span);
                let accessed = match &*value {
                    Value::Array(array) => match method.as_str() {
                        "first" => array.first().at(span)?,
                        "last" => array.last().at(span)?,
                        "at" => array.at(args.expect("index")?, None).at(span)?,
                        _ => return missing(),
                    },
                    Value::Dict(dict) => match method.as_str() {
                        "at" => dict.at(args.expect::<Str>("key")?, None).at(span)?,
                        _ => return missing(),
                    },
                    _ => return missing(),
                };

                Ok(Cow::Owned(accessed))
            }
        }
    }

    /// Gets the value using write access.
    pub fn write<'a>(
        &self,
        span: Span,
        vm: &'a mut VMState,
    ) -> SourceResult<&'a mut Value> {
        match self {
            Access::Readable(_) => {
                bail!(span, "cannot write to a readable, malformed access")
            }
            Access::Writable(writable) => vm.write(*writable).at(span),
            Access::Chained(value, field) => {
                let value = value.write(span, vm)?;
                match value {
                    Value::Dict(dict) => dict.at_mut(field.as_str()).at(span),
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
            Access::AccessorMethod(value, method, args) => {
                // Get the arguments.
                let args = vm.read(*args).at(span)?;
                let Value::Args(args) = args.clone() else {
                    bail!(span, "expected args, found {}", args.ty().long_name());
                };

                // Get the callee.
                let value = value.write(span, vm)?;

                call_method_access(value, method, args, span)
            }
        }
    }
}
/// The missing method error message.
#[cold]
fn missing_method(ty: Type, method: &str) -> String {
    format!("type {ty} has no method `{method}`")
}

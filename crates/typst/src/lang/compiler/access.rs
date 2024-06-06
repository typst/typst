use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;

use super::call::ArgsCompile;
use super::{Compile, Compiler, IntoCompiledValue, ReadableGuard, RegisterGuard};
use crate::diag::{bail, error, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{cannot_mutate_constant, unknown_variable, Func, IntoValue, Module, Type, Value};
use crate::lang::compiled::CompiledAccess;
use crate::lang::operands::AccessId;
use crate::utils::PicoStr;

#[derive(Debug, Clone, Hash)]
pub enum Access {
    /// Access this value through a register.
    Register(RegisterGuard),

    /// Access this value through a chained access.
    Chained(Span, Span, AccessId, PicoStr),

    /// Access a global value.
    Global(Module),

    /// A type that is accessed through a chain of accesses.
    Type(Type),

    /// Access this value through a chain of accesses.
    Value(Value),

    /// Access this value through an accessor method.
    Func(Func),

    /// Access this value through an accessor method.
    AccessorMethod(AccessId, PicoStr, ReadableGuard),
}

impl Access {
    /// Tries to resolve this access to a constant value.
    pub fn resolve(&self, compiler: &Compiler<'_>) -> SourceResult<Option<Value>> {
        match self {
            Access::Register(r) => {
                let Some(v) = compiler.resolve_var(r) else {
                    return Ok(None);
                };

                if !v.constant {
                    return Ok(None);
                }

                let default = compiler.resolve_default(r);
                Ok(default)
            }
            Access::Chained(_, _, other, v) => {
                let Some(access) = compiler.get_access(other) else {
                    return Ok(None);
                };

                let value = access.resolve(compiler)?;

                // We purposefully ignore missing fields because we want to avoid
                // issues with special methods on some types like `.with` on Func.
                Ok(value
                    .map(|access| access.field(v.resolve()))
                    .transpose()
                    .ok()
                    .flatten())
            }
            Access::Global(global) => Ok(Some(Value::Module(global.clone()))),
            Access::Type(ty) => Ok(Some(Value::Type(*ty))),
            Access::Value(value) => Ok(Some(value.clone())),
            Access::Func(func) => Ok(Some(Value::Func(func.clone()))),
            Access::AccessorMethod(_, _, _) => Ok(None),
        }
    }
}

impl IntoCompiledValue for Access {
    type CompiledValue = CompiledAccess;

    fn into_compiled_value(self) -> Self::CompiledValue {
        match self {
            Access::Register(r) => CompiledAccess::Register(r.into()),
            Access::Chained(parent_span, span, other, v) => {
                CompiledAccess::Chained(parent_span, other, v.resolve(), span)
            }
            Access::Global(global) => CompiledAccess::Module(global.into_value()),
            Access::Type(ty) => CompiledAccess::Type(ty.into_value()),
            Access::Value(value) => CompiledAccess::Value(value),
            Access::Func(func) => CompiledAccess::Func(func.into_value()),
            Access::AccessorMethod(other, v, r) => {
                CompiledAccess::AccessorMethod(other, v.resolve(), r.into())
            }
        }
    }
}

pub trait CompileAccess {
    /// Generate an access to the value.
    fn access(
        self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access>;
}

impl CompileAccess for ast::Expr<'_> {
    fn access(
        self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        match self {
            Self::Ident(v) => v.access(compiler, engine, mutable),
            Self::Parenthesized(v) => {
                v.access(compiler, engine, mutable)
            }
            Self::FieldAccess(v) => v.access(compiler, engine, mutable),
            Self::FuncCall(v) => v.access(compiler, engine, mutable),
            _ if mutable => {
                bail!(self.span(), "cannot mutate a temporary value");
            }
            other => {
                let register = compiler.allocate();

                // Even if we allocate an unnecessary register, it is still preferable for
                // easier implementation of accesses overall.
                other.compile(compiler, engine, register.clone().into())?;
                Ok(Access::Register(register))
            }
        }
    }
}

impl CompileAccess for ast::Ident<'_> {
    fn access(
        self,
        compiler: &mut Compiler,
        _: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        match compiler.read(self.span(), self.get(), mutable) {
            Some(ReadableGuard::Register(reg)) => {
                // Make a variable as no longer constant.
                if mutable {
                    compiler.mutate_variable(self.as_str());
                }

                Ok(Access::Register(reg))
            },
            Some(ReadableGuard::Captured(cap)) => {
                if mutable {
                    bail!(self.span(), "variables from outside the function are read-only and cannot be modified")
                } else {
                    Ok(Access::Register(cap.into()))
                }
            }
            Some(ReadableGuard::GlobalModule) => {
                if mutable {
                    return Err(cannot_mutate_constant(self.get())).at(self.span());
                } else {
                    Ok(Access::Global(compiler.library().global.clone()))
                }
            }
            Some(ReadableGuard::Global(global)) => {
                if mutable {
                    return Err(cannot_mutate_constant(self.get())).at(self.span());
                } else {
                    Ok(compiler
                        .library()
                        .global
                        .field_by_index(global.as_raw() as usize)
                        .ok_or_else(|| {
                            error!("could not find global `{}` in scope", self.get())
                        })
                        .at(self.span())?
                        .clone()
                        .into())
                }
            }
            None => {
                // Special case for constants.
                if mutable && compiler.library().global.field(self.get()).is_ok() {
                    return Err(cannot_mutate_constant(self.get())).at(self.span());
                }

                // Special case for

                return Err(unknown_variable(self.get())).at(self.span())
            },
            _ => bail!(self.span(), "unexpected variable access"),
        }
    }
}

impl CompileAccess for ast::Parenthesized<'_> {
    fn access(
        self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        self.expr().access(compiler, engine, mutable)
    }
}

impl CompileAccess for ast::FieldAccess<'_> {
    fn access(
        self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        let left = self.target().access(compiler, engine, mutable)?;
        let field = self.field().get();

        macro_rules! field {
            ($this:expr, $value:expr, $field:expr, $left:expr) => {{
                match $value.field($field) {
                    Ok(field) => field.clone(),
                    Err(_) => {
                        let left_id = compiler.access($left);
                        return Ok(Access::Chained(
                            $this.target().span(),
                            $this.field().span(),
                            left_id,
                            PicoStr::new(field),
                        ));
                    }
                }
            }};
        }

        Ok(match &left {
            Access::Global(global) => {
                Access::from(field!(self, global, field, left))
            }
            Access::Type(ty) => {
                Access::from(field!(self, ty, field, left))
            },
            Access::Func(func) => {
                Access::from(field!(self, func, field, left))
            },
            Access::Value(value) => {
                Access::from(field!(self, value, field, left))
            },
            _ => {
                let index = compiler.access(left);
                Access::Chained(
                    self.target().span(),
                    self.field().span(),
                    index,
                    PicoStr::new(field),
                )
            }
        })
    }
}

impl From<Value> for Access {
    fn from(value: Value) -> Self {
        match value {
            Value::Type(ty_) => Access::Type(ty_),
            Value::Func(func_) => Access::Func(func_),
            value => Access::Value(value),
        }
    }
}

impl CompileAccess for ast::FuncCall<'_> {
    fn access(
        self,
        compiler: &mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        if !mutable {
            // Compile the function call.
            let register = compiler.allocate();
            self.compile(compiler, engine, register.clone().into())?;

            Ok(Access::Register(register))
        } else if let ast::Expr::FieldAccess(access) = self.callee() {
            // Compile the arguments.
            let args = self.args();
            let args = args.compile_args(compiler, engine, self.span())?;

            // Ensure that the arguments live long enough.
            let left = access.target().access(compiler, engine, mutable)?;
            let index = compiler.access(left);

            let method = access.field();
            Ok(Access::AccessorMethod(index, PicoStr::new(method.get()), args))
        } else {
            bail!(self.span(), "cannot mutate a temporary value")
        }
    }
}

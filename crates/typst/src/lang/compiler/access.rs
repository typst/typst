use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;

use super::{Compile, Compiler, IntoCompiledValue, ReadableGuard, RegisterGuard};
use crate::diag::{bail, error, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Func, IntoValue, Module, Type, Value};
use crate::lang::compiled::CompiledAccess;
use crate::lang::operands::AccessId;
use crate::utils::PicoStr;

#[derive(Debug, Clone, Hash)]
pub enum Access {
    /// Access this value through a readable.
    Readable(ReadableGuard),

    /// Access this value through a writeable.
    Writable(RegisterGuard),

    /// Access this value through a chained access.
    Chained(Span, AccessId, PicoStr),

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
            Access::Readable(r) => {
                let ReadableGuard::Register(r) = r else {
                    return Ok(None);
                };

                let Some(v) = compiler.resolve_var(r) else {
                    return Ok(None);
                };

                if !v.constant {
                    return Ok(None);
                }

                Ok(v.default)
            }
            Access::Writable(_) => Ok(None),
            Access::Chained(span, other, v) => {
                let Some(access) = compiler.get_access(other) else {
                    return Ok(None);
                };

                access
                    .resolve(compiler)?
                    .map(|access| access.field(v.resolve()))
                    .transpose()
                    .at(*span)
            }
            Access::Global(global) => Ok(Some(Value::Module(global.clone()))),
            Access::Type(ty) => Ok(Some(Value::Type(ty.clone()))),
            Access::Value(value) => Ok(Some(value.clone())),
            Access::Func(func) => Ok(Some(Value::Func(func.clone()))),
            Access::AccessorMethod(_, _, _) => Ok(None),
        }
    }

    /*pub fn as_vm_access(&self) -> VmAccess {
        match self {
            AccessPattern::Readable(r) => VmAccess::Readable(r.as_readable()),
            AccessPattern::Writable(w) => VmAccess::Writable(w.as_writable()),
            AccessPattern::Chained(other, v) => {
                VmAccess::Chained(Arc::new(other.as_vm_access()), v.clone())
            }
            AccessPattern::Global(global) => {
                VmAccess::Module(global.clone().into_value())
            }
            AccessPattern::AccessorMethod(other, v, r) => VmAccess::AccessorMethod(
                Arc::new(other.as_vm_access()),
                v.clone(),
                r.as_readable(),
            ),
            AccessPattern::Type(ty) => VmAccess::Type(ty.clone().into_value()),
            AccessPattern::Value(value) => VmAccess::Value(value.clone()),
            AccessPattern::Func(func) => VmAccess::Func(func.clone().into_value()),
        }
    }*/
}

impl IntoCompiledValue for Access {
    type CompiledValue = CompiledAccess;

    fn into_compiled_value(self) -> Self::CompiledValue {
        match self {
            Access::Readable(r) => CompiledAccess::Readable(r.into()),
            Access::Writable(w) => CompiledAccess::Writable(w.into()),
            Access::Chained(_, other, v) => CompiledAccess::Chained(other, v),
            Access::Global(global) => CompiledAccess::Module(global.into_value()),
            Access::Type(ty) => CompiledAccess::Type(ty.into_value()),
            Access::Value(value) => CompiledAccess::Value(value),
            Access::Func(func) => CompiledAccess::Func(func.into_value()),
            Access::AccessorMethod(other, v, r) => {
                CompiledAccess::AccessorMethod(other, v, r.into())
            }
        }
    }
}

pub trait CompileAccess {
    /// Generate an access to the value.
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access>;
}

impl CompileAccess for ast::Expr<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        match self {
            Self::Ident(v) => v.access(compiler, engine, mutable),
            Self::Parenthesized(v) => v.access(compiler, engine, mutable),
            Self::FieldAccess(v) => v.access(compiler, engine, mutable),
            Self::FuncCall(v) => v.access(compiler, engine, mutable),
            _ if mutable => {
                bail!(self.span(), "cannot mutate a temporary value");
            }
            other => {
                let value = other.compile_to_readable(compiler, engine)?;
                Ok(Access::Readable(value))
            }
        }
    }
}

impl CompileAccess for ast::Ident<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        _: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        match compiler.read(self.span(), self.get(), mutable) {
            Some(ReadableGuard::Register(reg)) => {
                if mutable {
                    Ok(Access::Writable(reg))
                } else {
                    Ok(Access::Readable(reg.into()))
                }
            }
            Some(ReadableGuard::Captured(cap)) => {
                if mutable {
                    bail!(self.span(), "variables from outside the function are read-only and cannot be modified")
                } else {
                    Ok(Access::Readable(*cap))
                }
            }
            Some(ReadableGuard::Global(global)) => {
                if mutable {
                    bail!(self.span(), "variables in the global scope are read-only and cannot be modified")
                } else {
                    match compiler
                        .library()
                        .global
                        .field_by_index(global.as_raw() as usize)
                        .ok_or_else(|| {
                            error!("could not find global `{}` in scope", self.get())
                        })
                        .at(self.span())?
                    {
                        Value::Module(module) => Ok(Access::Global(module.clone())),
                        Value::Type(ty_) => Ok(Access::Type(ty_.clone())),
                        Value::Func(func_) => Ok(Access::Func(func_.clone())),
                        value => Ok(Access::Value(value.clone())),
                    }
                }
            }
            None => bail!(self.span(), "could not find `{}` in scope", self.get()),
            _ => unreachable!(),
        }
    }
}

impl CompileAccess for ast::Parenthesized<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        self.expr().access(compiler, engine, mutable)
    }
}

impl CompileAccess for ast::FieldAccess<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        let left = self.target().access(compiler, engine, mutable)?;
        match left {
            Access::Global(global) => {
                match global.field(self.field().get()).at(self.span())? {
                    Value::Module(module) => Ok(Access::Global(module.clone())),
                    Value::Type(ty_) => Ok(Access::Type(ty_.clone())),
                    Value::Func(func_) => Ok(Access::Func(func_.clone())),
                    value => Ok(Access::Value(value.clone())),
                }
            }
            Access::Type(ty) => {
                match ty.field(self.field().get()).at(self.field().span())? {
                    Value::Module(module) => Ok(Access::Global(module.clone())),
                    Value::Type(ty_) => Ok(Access::Type(ty_.clone())),
                    Value::Func(func_) => Ok(Access::Func(func_.clone())),
                    value => Ok(Access::Value(value.clone())),
                }
            }
            Access::Func(func) => {
                match func.field(self.field().get()).at(self.field().span())? {
                    Value::Module(module) => Ok(Access::Global(module.clone())),
                    Value::Type(ty_) => Ok(Access::Type(ty_.clone())),
                    Value::Func(func_) => Ok(Access::Func(func_.clone())),
                    value => Ok(Access::Value(value.clone())),
                }
            }
            Access::Value(value) => {
                match value.field(self.field().get()).at(self.field().span())? {
                    Value::Module(module) => Ok(Access::Global(module.clone())),
                    Value::Type(ty_) => Ok(Access::Type(ty_.clone())),
                    Value::Func(func_) => Ok(Access::Func(func_.clone())),
                    value => Ok(Access::Value(value.clone())),
                }
            }
            other => {
                let index = compiler.access(other);
                Ok(Access::Chained(self.span(), index, PicoStr::new(self.field().get())))
            }
        }
    }
}

impl CompileAccess for ast::FuncCall<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        engine: &mut Engine,
        mutable: bool,
    ) -> SourceResult<Access> {
        if !mutable {
            // Compile the function call.
            let call = self.compile_to_readable(compiler, engine)?;
            Ok(Access::Readable(call))
        } else if let ast::Expr::FieldAccess(access) = self.callee() {
            // Compile the arguments.
            let args = self.args();
            let args = args.compile_to_readable(compiler, engine)?;

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

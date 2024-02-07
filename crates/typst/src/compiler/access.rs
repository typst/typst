use std::sync::Arc;

use ecow::EcoString;
use typst_syntax::ast::{self, AstNode};

use super::{Compile, Compiler, ReadableGuard, WritableGuard};
use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Func, IntoValue, Module, Type, Value};
use crate::vm::Access as VmAccess;

#[derive(Debug, Clone)]
pub enum AccessPattern {
    /// Access this value through a readable.
    Readable(ReadableGuard),

    /// Access this value through a writeable.
    Writable(WritableGuard),

    /// Access this value through a chained access.
    Chained(Arc<Self>, EcoString),

    /// Access a global value.
    Global(Module),

    /// A type that is accessed through a chain of accesses.
    Type(Type),

    /// Access this value through a chain of accesses.
    Value(Value),

    /// Access this value through an accessor method.
    Func(Func),

    /// Access this value through an accessor method.
    AccessorMethod(Arc<Self>, EcoString, ReadableGuard),
}

impl AccessPattern {
    pub fn as_vm_access(&self) -> VmAccess {
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
    }
}

pub trait Access {
    /// Generate an access to the value.
    fn access<'a>(
        self,
        engine: &mut Engine,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern>;
}

impl Access for ast::Expr<'_> {
    fn access<'a>(
        self,
        engine: &mut Engine,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        match self {
            Self::Ident(v) => v.access(engine, compiler, mutable),
            Self::Parenthesized(v) => v.access(engine, compiler, mutable),
            Self::FieldAccess(v) => v.access(engine, compiler, mutable),
            Self::FuncCall(v) => v.access(engine, compiler, mutable),
            _ if mutable => {
                bail!(self.span(), "cannot mutate a temporary value");
            }
            other => {
                let value = other.compile(engine, compiler)?;
                Ok(AccessPattern::Readable(value))
            }
        }
    }
}

impl Access for ast::Ident<'_> {
    fn access<'a>(
        self,
        _: &mut Engine,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        match compiler.read(self.span(), self.get()).at(self.span())? {
            Some(ReadableGuard::Register(reg)) => {
                if mutable {
                    Ok(AccessPattern::Writable(reg.into()))
                } else {
                    Ok(AccessPattern::Readable(reg.into()))
                }
            }
            Some(ReadableGuard::Captured(cap)) => {
                if mutable {
                    bail!(self.span(), "variables from outside the function are read-only and cannot be modified")
                } else {
                    Ok(AccessPattern::Readable((*cap).into()))
                }
            }
            Some(ReadableGuard::Global(global)) => {
                if mutable {
                    bail!(self.span(), "variables in the global scope are read-only and cannot be modified")
                } else {
                    match compiler
                        .library()
                        .global
                        .field_by_id(global.as_raw() as usize)
                        .at(self.span())?
                    {
                        Value::Module(module) => {
                            Ok(AccessPattern::Global(module.clone()))
                        }
                        Value::Type(ty_) => Ok(AccessPattern::Type(ty_.clone())),
                        Value::Func(func_) => Ok(AccessPattern::Func(func_.clone())),
                        value => Ok(AccessPattern::Value(value.clone())),
                    }
                }
            }
            None => bail!(self.span(), "could not find `{}` in scope", self.get()),
            _ => unreachable!(),
        }
    }
}

impl Access for ast::Parenthesized<'_> {
    fn access<'a>(
        self,
        engine: &mut Engine,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        self.expr().access(engine, compiler, mutable)
    }
}

impl Access for ast::FieldAccess<'_> {
    fn access<'a>(
        self,
        engine: &mut Engine,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        let left = self.target().access(engine, compiler, mutable)?;
        match left {
            AccessPattern::Global(global) => {
                match global.field(self.field().get()).at(self.span())? {
                    Value::Module(module) => Ok(AccessPattern::Global(module.clone())),
                    Value::Type(ty_) => Ok(AccessPattern::Type(ty_.clone())),
                    Value::Func(func_) => Ok(AccessPattern::Func(func_.clone())),
                    value => Ok(AccessPattern::Value(value.clone())),
                }
            }
            AccessPattern::Type(ty) => {
                match ty.field(self.field().get()).at(self.field().span())? {
                    Value::Module(module) => Ok(AccessPattern::Global(module.clone())),
                    Value::Type(ty_) => Ok(AccessPattern::Type(ty_.clone())),
                    Value::Func(func_) => Ok(AccessPattern::Func(func_.clone())),
                    value => Ok(AccessPattern::Value(value.clone())),
                }
            }
            AccessPattern::Func(func) => {
                match func.field(self.field().get()).at(self.field().span())? {
                    Value::Module(module) => Ok(AccessPattern::Global(module.clone())),
                    Value::Type(ty_) => Ok(AccessPattern::Type(ty_.clone())),
                    Value::Func(func_) => Ok(AccessPattern::Func(func_.clone())),
                    value => Ok(AccessPattern::Value(value.clone())),
                }
            }
            AccessPattern::Value(value) => {
                match value.field(self.field().get()).at(self.field().span())? {
                    Value::Module(module) => Ok(AccessPattern::Global(module.clone())),
                    Value::Type(ty_) => Ok(AccessPattern::Type(ty_.clone())),
                    Value::Func(func_) => Ok(AccessPattern::Func(func_.clone())),
                    value => Ok(AccessPattern::Value(value.clone())),
                }
            }
            other => {
                Ok(AccessPattern::Chained(Arc::new(other), self.field().get().clone()))
            }
        }
    }
}

impl Access for ast::FuncCall<'_> {
    fn access<'a>(
        self,
        engine: &mut Engine,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        if !mutable {
            // Compile the function call.
            let call = self.compile(engine, compiler)?;
            Ok(AccessPattern::Readable(call))
        } else if let ast::Expr::FieldAccess(access) = self.callee() {
            // Compile the arguments.
            let args = self.args();
            let args = args.compile(engine, compiler)?;

            // Ensure that the arguments live long enough.
            let left = access.target().access(engine, compiler, mutable)?;

            let method = access.field();
            Ok(AccessPattern::AccessorMethod(Arc::new(left), method.get().clone(), args))
        } else {
            bail!(self.span(), "cannot mutate a temporary value")
        }
    }
}

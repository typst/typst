use std::sync::Arc;

use ecow::EcoString;
use typst_syntax::ast::{self, AstNode};

use super::{Compile, Compiler, ReadableGuard, WritableGuard};
use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::vm::Access as VmAccess;

#[derive(Debug, Clone)]
pub enum AccessPattern {
    /// Access this value through a readable.
    Readable(ReadableGuard),

    /// Access this value through a writeable.
    Writable(WritableGuard),

    /// Access this value through a chained access.
    Chained(Arc<Self>, EcoString),

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
            AccessPattern::AccessorMethod(other, v, r) => VmAccess::AccessorMethod(
                Arc::new(other.as_vm_access()),
                v.clone(),
                r.as_readable(),
            ),
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
            Some(ReadableGuard::Parent(parent)) => {
                if mutable {
                    Ok(AccessPattern::Writable(parent.into()))
                } else {
                    Ok(AccessPattern::Readable(parent.into()))
                }
            }
            Some(ReadableGuard::Captured(cap)) => {
                if mutable {
                    bail!(self.span(), "cannot mutate a captured value")
                } else {
                    Ok(AccessPattern::Readable((*cap).into()))
                }
            }
            Some(ReadableGuard::Global(global)) => {
                if mutable {
                    bail!(self.span(), "cannot mutate a global value")
                } else {
                    Ok(AccessPattern::Readable(global.into()))
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
        Ok(AccessPattern::Chained(Arc::new(left), self.field().get().clone()))
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

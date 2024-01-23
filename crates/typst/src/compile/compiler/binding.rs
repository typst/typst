use ecow::EcoString;
use typst_syntax::ast::{self, AstNode};

use crate::compile::destructure::PatternCompile;
use crate::compile::{
    CallId, Compile, Compiler, Instruction, LocalId, PatternItem, PatternKind, Register,
    ScopeId,
};
use crate::diag::{bail, At, SourceResult};

impl Compile for ast::LetBinding<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        match self.kind() {
            ast::LetBindingKind::Normal(pattern) => {
                // We compile the initializer.
                let value = if let Some(init) = self.init() {
                    init.compile(compiler)?
                } else {
                    Register::NONE
                };

                // We compile the pattern.
                let pattern = pattern.compile(compiler, true)?;

                // We destructure the initializer using the pattern.
                // Simple patterns can be directly stored.
                if let PatternKind::Single(PatternItem::Simple(
                    span,
                    AccessPattern::Local(ScopeId(0), id),
                    _,
                )) = &pattern.kind
                {
                    compiler.spans.push(*span);
                    compiler.instructions.push(Instruction::Store { scope: ScopeId::SELF, local: *id, value });
                } else {
                    // We add the pattern to the local scope.
                    let pattern_id = compiler.pattern(pattern);

                    // Otherwise we destructure the initializer.
                    compiler.spans.push(self.span());
                    compiler
                        .instructions
                        .push(Instruction::Destructure { pattern: pattern_id, value });
                }

                compiler.free(value);

                // We do not produce a value.
                Ok(Register::NONE)
            }
            ast::LetBindingKind::Closure(name) => {
                // We create the local.
                let local = compiler.local(name.span(), name.get().clone());

                // We compile the initializer.
                let value = if let Some(init) = self.init() {
                    let mut name = Some(name.get().clone());
                    std::mem::swap(&mut compiler.current_name, &mut name);
                    let res = init.compile(compiler)?;
                    std::mem::swap(&mut compiler.current_name, &mut name);
                    res
                } else {
                    Register::NONE
                };

                // We set the local to the initializer.
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Store { scope: ScopeId::SELF, local, value });

                compiler.free(value);

                // We do not produce a value.
                Ok(Register::NONE)
            }
        }
    }
}

impl Compile for ast::DestructAssignment<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        // We compile the pattern and add it to the local scope.
        let pattern = self.pattern().compile(compiler, false)?;

        // We compile the initializer.
        let value = self.value().compile(compiler)?;

        // We destructure the initializer using the pattern.
        if let PatternKind::Single(PatternItem::Simple(
            span,
            AccessPattern::Local(ScopeId(0), id),
            _,
        )) = &pattern.kind
        {
            compiler.spans.push(*span);
            compiler.instructions.push(Instruction::Store { scope: ScopeId::SELF, local: *id, value });
        } else {
            let pattern_id = compiler.pattern(pattern);
            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::Destructure { pattern: pattern_id, value });
        }

        compiler.free(value);

        // We do not produce a value.
        Ok(Register::NONE)
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub enum AccessPattern {
    Register(Register),
    Local(ScopeId, LocalId),
    Chained(Box<AccessPattern>, EcoString),
    AccessorMethod(Box<AccessPattern>, CallId, EcoString),
}

impl AccessPattern {
    pub fn free(&self, compiler: &mut Compiler) {
        match self {
            AccessPattern::Register(register) => compiler.free(*register),
            AccessPattern::Local(_, _) => {}
            AccessPattern::Chained(left, _) => left.free(compiler),
            AccessPattern::AccessorMethod(left, call, _) => {
                left.free(compiler);

                let call = compiler.calls[call.0 as usize].clone();
                call.free(compiler);
            }
        }
    }
}

pub trait Access {
    /// Generate an access to the value.
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern>;
}

impl Access for ast::Expr<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        match self {
            Self::Ident(v) => v.access(compiler, mutable),
            Self::Parenthesized(v) => v.access(compiler, mutable),
            Self::FieldAccess(v) => v.access(compiler, mutable),
            Self::FuncCall(v) => v.access(compiler, mutable),
            _ => {
                bail!(self.span(), "cannot mutate a temporary value");
            }
        }
    }
}

impl Access for ast::Ident<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        _: bool,
    ) -> SourceResult<AccessPattern> {
        match compiler.local_ref(self.get(), Register::NONE) {
            Some(Instruction::Load { scope, local, .. }) => {
                Ok(AccessPattern::Local(scope, local))
            }
            Some(Instruction::LoadModule { .. }) => {
                bail!(self.span(), "cannot mutate an imported value")
            }
            Some(Instruction::LoadCaptured { .. }) => {
                bail!(self.span(), "cannot mutate a captured value")
            }
            None => bail!(self.span(), "could not find `{}` in scope", self.get()),
            _ => unreachable!(),
        }
    }
}

impl Access for ast::Parenthesized<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        self.expr().access(compiler, mutable)
    }
}

impl Access for ast::FieldAccess<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        let left = self.target().access(compiler, mutable)?;
        Ok(AccessPattern::Chained(Box::new(left), self.field().get().clone()))
    }
}

impl Access for ast::FuncCall<'_> {
    fn access<'a>(
        self,
        compiler: &'a mut Compiler,
        mutable: bool,
    ) -> SourceResult<AccessPattern> {
        if let ast::Expr::FieldAccess(access) = self.callee() {
            self.compile(compiler)?;

            // Remove the actual call.
            compiler.spans.pop();
            let Some(Instruction::Call { call }) = compiler.instructions.pop() else {
                bail!(self.span(), "expected a call instruction");
            };

            // Ensure that the arguments live long enough.
            let args = compiler.calls[call.0 as usize].args;
            compiler.use_reg(args).at(access.span())?;

            let left = access.target().access(compiler, mutable)?;

            let method = access.field();
            Ok(AccessPattern::AccessorMethod(Box::new(left), call, method.get().clone()))
        } else if !mutable {
            let callee = self.callee().compile(compiler)?;
            Ok(AccessPattern::Register(callee))
        } else {
            bail!(self.span(), "cannot mutate a temporary value")
        }
    }
}

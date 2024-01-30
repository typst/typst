use typst_syntax::ast::{self, AstNode};

use crate::compile::destructure::PatternCompile;
use crate::compile::{PatternItem, PatternKind, ScopeId};
use crate::diag::{bail, At, SourceResult};

use super::{AccessPattern, Compile, Compiler, Instruction, Register};

impl Compile for ast::Conditional<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let condition = self.condition().compile(compiler)?;

        if let Some(else_body) = self.else_body() {
            let true_ = compiler.label();
            let after = compiler.label();

            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::JumpIf { condition, label: true_ });

            let else_body = else_body.compile(compiler)?;

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Jump { label: after });

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: true_ });

            let if_body = self.if_body().compile(compiler)?;

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: after });

            if if_body.is_none() && else_body.is_none() {
                compiler.free(condition);
                return Ok(Register::NONE);
            }

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Select {
                condition,
                lhs: if_body,
                rhs: else_body,
                target: if_body,
            });

            compiler.free(condition);
            compiler.free(else_body);

            Ok(if_body)
        } else {
            let after = compiler.label();

            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::JumpIfNot { condition, label: after });

            let if_body = self.if_body().compile(compiler)?;

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: after });

            if if_body.is_none() {
                return Ok(Register::NONE);
            }

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Select {
                condition,
                lhs: if_body,
                rhs: Register::NONE,
                target: if_body,
            });

            compiler.free(condition);

            Ok(if_body)
        }
    }
}

impl Compile for ast::WhileLoop<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::JoinGroup { content: false, capacity: 0 });

        compiler.in_scope(self.span(), |compiler| {
            let top = compiler.label();
            let bottom = compiler.label();

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: top });

            let condition = self.condition().compile(compiler)?;

            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::JumpIfNot { condition, label: bottom });

            compiler.loop_stack.push((top, bottom, compiler.scopes.len()));

            let out = self.body().compile(compiler)?;
            if !out.is_none() {
                compiler.spans.push(self.body().span());
                compiler.instructions.push(Instruction::Join { value: out });
            }

            compiler.loop_stack.pop();

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Jump { label: top });

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: bottom });

            Ok(())
        })?;

        let target = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::PopGroup { target, content: false });

        Ok(target)
    }
}

impl Compile for ast::ForLoop<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::JoinGroup { content: false, capacity: 0 });

        compiler.in_scope(self.span(), |compiler| {
            let top = compiler.label();
            let bottom = compiler.label();
            let iterator = compiler.iterator();

            let iterable = self.iter().compile(compiler)?;
            let next = compiler.reg().at(self.span())?;

            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::Iter { value: iterable, iterator });

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: top });
            compiler.free(iterable);

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Next {
                iterator,
                target: next,
                exhausted: bottom,
            });

            let pattern = self.pattern().compile(compiler, true)?;

            if let PatternKind::Single(PatternItem::Simple(
                span,
                AccessPattern::Local(ScopeId::SELF, id),
                _,
            )) = &pattern.kind
            {
                compiler.spans.push(*span);
                compiler.instructions.push(Instruction::Store {
                    scope: ScopeId::SELF,
                    local: *id,
                    value: next,
                });
            } else {
                let pattern_id = compiler.pattern(pattern.clone());
                compiler.spans.push(self.pattern().span());
                compiler
                    .instructions
                    .push(Instruction::Destructure { pattern: pattern_id, value: next });
            }

            compiler.loop_stack.push((top, bottom, compiler.scopes.len()));
            let res = self.body().compile(compiler)?;
            if !res.is_none() {
                compiler.spans.push(self.body().span());
                compiler.instructions.push(Instruction::Join { value: res });
            }

            pattern.free(compiler);
            compiler.free(res);
            compiler.loop_stack.pop();

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Jump { label: top });

            compiler.pop_iterator();

            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Label { label: bottom });

            compiler.free(next);

            Ok(())
        })?;

        let target = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::PopGroup { target, content: false });
        Ok(target)
    }
}

impl Compile for ast::LoopBreak<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let Some((_, bottom, stack)) = compiler.loop_stack.last().copied() else {
            bail!(self.span(), "break outside of loop")
        };

        for _ in stack..compiler.scopes.len() {
            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Exit {});
        }

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Jump { label: bottom });

        Ok(Register::NONE)
    }
}

impl Compile for ast::LoopContinue<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let Some((top, _, stack)) = compiler.loop_stack.last().copied() else {
            bail!(self.span(), "continue outside of loop")
        };

        for _ in stack..compiler.scopes.len() {
            compiler.spans.push(self.span());
            compiler.instructions.push(Instruction::Exit {});
        }

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Jump { label: top });

        Ok(Register::NONE)
    }
}

use std::num::NonZeroU16;

use ecow::eco_vec;
use typst_syntax::ast::{self, AstNode};

use crate::diag::{error, At, SourceResult};
use crate::foundations::{IntoValue, Label, NativeElement, Value};
use crate::model::{LinkElem, ParbreakElem};
use crate::symbols::Symbol;
use crate::text::{LinebreakElem, RawElem, SmartQuoteElem};
use crate::text::{SpaceElem, TextElem};

use super::{Compile, Compiler, Instruction, Register};

impl Compile for ast::Markup<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        // In order to keep track of styles, every time we encounter a row, we
        // push a new style context onto the stack. This context is popped when
        // we encounter a row end.
        let mut stack_depth = 0;

        // The markup is a sequence of expressions.
        let exprs = self.exprs();

        // We push a new join group.
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::JoinGroup {
            capacity: exprs.size_hint().1.unwrap_or_else(|| exprs.size_hint().0) as u16,
        });

        for expr in exprs {
            // Handle set rules specially.
            if let ast::Expr::Set(set) = expr {
                let style = set.compile(compiler)?;

                stack_depth += 1;
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::StylePush { style });
                compiler.free(style);

                continue;
            }

            // Handle show rules specially.
            if let ast::Expr::Show(show) = expr {
                let style = show.compile(compiler)?;

                stack_depth += 1;
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::StylePush { style });
                compiler.free(style);

                continue;
            }

            let expr = expr.compile_display(compiler)?;
            if !expr.is_none() {
                compiler.spans.push(self.span());
                compiler.instructions.push(Instruction::Join { value: expr });
            }
            compiler.free(expr);
        }

        if stack_depth > 0 {
            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::StylePop { depth: stack_depth });
        }

        // We pop the join group.
        compiler.spans.push(self.span());

        // We assign a register to the result of the markup.
        let res = compiler.reg().at(self.span())?;
        compiler
            .instructions
            .push(Instruction::PopGroup { target: res, content: true });

        Ok(res)
    }
}

impl Compile for ast::Text<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = TextElem::new(self.get().clone()).pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Space<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = SpaceElem::new().pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Linebreak<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = LinebreakElem::new().pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Parbreak<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = ParbreakElem::new().pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Escape<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = Value::Symbol(Symbol::single(self.get()));
        let value = compiler.const_(value);
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Shorthand<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = Value::Symbol(Symbol::single(self.get()));
        let value = compiler.const_(value);
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::SmartQuote<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = SmartQuoteElem::new()
            .with_double(self.double())
            .pack()
            .spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Strong<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let body = self.body().compile(compiler)?;
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::Strong { value: body, target: register });

        compiler.free(body);

        Ok(register)
    }
}

impl Compile for ast::Emph<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let body = self.body().compile(compiler)?;
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::Emph { value: body, target: register });

        compiler.free(body);

        Ok(register)
    }
}

impl Compile for ast::Raw<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let mut elem = RawElem::new(self.text()).with_block(self.block());
        if let Some(lang) = self.lang() {
            elem.push_lang(Some(lang.into()));
        }

        let value = elem.pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Link<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = LinkElem::from_url(self.get().clone()).pack().spanned(self.span());
        let value = compiler.const_(value.into_value());
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Label<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let value = Value::Label(Label::new(self.get()));
        let value = compiler.const_(value);
        let register = compiler.reg().at(self.span())?;

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { value, register });

        Ok(register)
    }
}

impl Compile for ast::Ref<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let label = compiler.content_label(self.target());
        let supplement = self
            .supplement()
            .map_or(Ok(Register::NONE), |value| value.compile(compiler))?;

        let register = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Ref {
            label,
            supplement,
            target: register,
        });

        compiler.free(supplement);

        Ok(register)
    }
}

impl Compile for ast::Heading<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let level = self.level();
        let body = self.body().compile(compiler)?;

        let register = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Heading {
            level: NonZeroU16::try_from(level).map_err(|_| {
                eco_vec![error!(self.span(), "heading level must be between 1 and 65535")]
            })?,
            body,
            target: register,
        });

        compiler.free(body);

        Ok(register)
    }
}

impl Compile for ast::ListItem<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let item = self.body().compile(compiler)?;

        let register = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::ListItem { item, target: register });

        compiler.free(item);

        Ok(register)
    }
}

impl Compile for ast::EnumItem<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let number = self.number().map(|n| n as u16);
        let item = self.body().compile(compiler)?;

        let register = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::EnumItem {
            number,
            item,
            target: register,
        });

        compiler.free(item);

        Ok(register)
    }
}

impl Compile for ast::TermItem<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let term = self.term().compile(compiler)?;
        let description = self.description().compile(compiler)?;

        let register = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::TermItem {
            term,
            description,
            target: register,
        });

        compiler.free(term);
        compiler.free(description);

        Ok(register)
    }
}

impl Compile for ast::Equation<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let body = self.body().compile(compiler)?;

        let register = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Equation {
            block: self.block(),
            body,
            target: register,
        });

        compiler.free(body);

        Ok(register)
    }
}

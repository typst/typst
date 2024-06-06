use std::num::{NonZeroU16, NonZeroU32};

use typst_syntax::ast::{self, AstNode};

use crate::diag::{error, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Content, Label, NativeElement, Value};
use crate::model::{LinkElem, ParbreakElem, RefElem};
use crate::symbols::Symbol;
use crate::text::{
    LinebreakElem, RawContent, RawElem, SmartQuoteElem, SpaceElem, TextElem,
};

use super::{
    copy_constant, Compile, CompileTopLevel, Compiler, ReadableGuard, WritableGuard,
};

impl CompileTopLevel for ast::Markup<'_> {
    fn compile_top_level(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        let mut iter = self.exprs();

        // Special case to avoid creating empty scope.
        let Some(first) = iter.next() else {
            let const_id = compiler.const_(Content::empty());

            // Copy an empty content to the output.
            compiler.copy(self.span(), const_id, WritableGuard::Joined);
            compiler.flow();

            return Ok(());
        };

        for expr in std::iter::once(first).chain(iter) {
            // Handle set rules specially.
            if let ast::Expr::Set(set) = expr {
                set.compile(compiler, engine, WritableGuard::Joined)?;
                compiler.flow();
                continue;
            }

            // Handle show rules specially.
            if let ast::Expr::Show(show) = expr {
                show.compile(compiler, engine, WritableGuard::Joined)?;
                compiler.flow();
                continue;
            }

            // Compile the expression, appending its output to the joiner.
            expr.compile(compiler, engine, WritableGuard::Joined)?;
            compiler.flow();
        }

        Ok(())
    }
}

impl Compile for ast::Markup<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        // Special case to avoid creating empty scope.
        if self.exprs().next().is_none() {
            let const_id = compiler.const_(Content::empty());

            // Copy an empty content to the output.
            compiler.copy(self.span(), const_id, output);
            compiler.flow();

            return Ok(());
        }

        compiler.enter(engine, self.span(), output, |compiler, engine| {
            self.compile_top_level(compiler, engine).map(|_| true)
        })
    }
}

impl Compile for ast::Text<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let text_elem = TextElem::new(self.get().clone()).pack().spanned(self.span());

        Ok(compiler.const_(text_elem).into())
    }
}

impl Compile for ast::Space<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let space_elem = SpaceElem::new().pack();

        Ok(compiler.const_(space_elem).into())
    }
}

impl Compile for ast::Linebreak<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let parbreak_elem = LinebreakElem::new().pack();

        Ok(compiler.const_(parbreak_elem).into())
    }
}

impl Compile for ast::Parbreak<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let parbreak_elem = ParbreakElem::new().pack();

        Ok(compiler.const_(parbreak_elem).into())
    }
}

impl Compile for ast::Escape<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let symbol = Value::Symbol(Symbol::single(self.get()));

        Ok(compiler.const_(symbol).into())
    }
}

impl Compile for ast::Shorthand<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let symbol = Value::Symbol(Symbol::single(self.get()));

        Ok(compiler.const_(symbol).into())
    }
}

impl Compile for ast::SmartQuote<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let smart_quote_elem = SmartQuoteElem::new()
            .with_double(self.double())
            .pack()
            .spanned(self.span());

        Ok(compiler.const_(smart_quote_elem).into())
    }
}

impl Compile for ast::Strong<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let body = self.body().compile_to_readable(compiler, engine)?;
        compiler.strong(self.span(), body, output);

        Ok(())
    }
}

impl Compile for ast::Emph<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let body = self.body().compile_to_readable(compiler, engine)?;
        compiler.emph(self.span(), body, output);

        Ok(())
    }
}

impl Compile for ast::Raw<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let lines = self.lines().map(|line| (line.get().clone(), line.span())).collect();
        let mut elem = RawElem::new(RawContent::Lines(lines)).with_block(self.block());
        if let Some(lang) = self.lang() {
            elem.push_lang(Some(lang.get().clone()));
        }

        Ok(compiler.const_(elem.pack()).into())
    }
}

impl Compile for ast::Link<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let link_elem = LinkElem::from_url(self.get().clone());

        Ok(compiler.const_(link_elem).into())
    }
}

impl Compile for ast::Label<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        copy_constant!(self, compiler, engine, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let label = Label::new(self.get());
        Ok(compiler.label(label).into())
    }
}

impl Compile for ast::Ref<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        // We can turn the reference into a constant if it has no supplement.
        // This has the advantage of pre-allocating the reference.
        let Some(supplement) = self
            .supplement()
            .map(|sup| sup.compile_to_readable(compiler, engine))
            .transpose()?
        else {
            let ref_elem =
                RefElem::new(Label::new(self.target())).pack().spanned(self.span());
            let constant = compiler.const_(ref_elem);

            compiler.copy(self.span(), constant, output);

            return Ok(());
        };

        let label = compiler.label(Label::new(self.target()));
        compiler.ref_(self.span(), label, supplement, output);

        Ok(())
    }

    fn compile_to_readable(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        // We can turn the reference into a constant if it has no supplement.
        // This has the advantage of pre-allocating the reference.
        let Some(supplement) = self
            .supplement()
            .map(|sup| sup.compile_to_readable(compiler, engine))
            .transpose()?
        else {
            let ref_elem =
                RefElem::new(Label::new(self.target())).pack().spanned(self.span());
            let constant = compiler.const_(ref_elem);

            return Ok(constant.into());
        };

        let label = compiler.label(Label::new(self.target()));
        let register = compiler.allocate();
        compiler.ref_(self.span(), label, supplement, register.clone());

        Ok(register.into())
    }
}

impl Compile for ast::Heading<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let level = self.depth();
        let body = self.body().compile_to_readable(compiler, engine)?;

        // This error is highly unlikely to happen.
        let level = NonZeroU16::try_from(level).map_err(|_| {
            vec![error!(self.span(), "level is too big: {level} > {}", u32::MAX)]
        })?;

        compiler.heading(self.span(), body, level, output);

        Ok(())
    }
}

impl Compile for ast::ListItem<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let body = self.body().compile_to_readable(compiler, engine)?;

        compiler.list_item(self.span(), body, output);

        Ok(())
    }
}

impl Compile for ast::EnumItem<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let number = self.number().and_then(|n| NonZeroU32::new(n as u32 + 1));
        let body = self.body().compile_to_readable(compiler, engine)?;

        compiler.enum_item(self.span(), body, number, output);

        Ok(())
    }
}

impl Compile for ast::TermItem<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let term = self.term().compile_to_readable(compiler, engine)?;
        let description = self.description().compile_to_readable(compiler, engine)?;

        compiler.term_item(self.span(), term, description, output);

        Ok(())
    }
}

impl Compile for ast::Equation<'_> {
    fn compile(
        &self,
        compiler: &mut Compiler<'_>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let body = self.body().compile_to_readable(compiler, engine)?;
        compiler.equation(self.span(), body, self.block(), output);

        Ok(())
    }
}

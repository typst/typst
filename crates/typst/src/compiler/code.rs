use std::sync::Arc;

use typst_syntax::ast::{self, AstNode};

use crate::compiler::{Access, AccessPattern};
use crate::diag::{bail, error, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::Value;
use crate::vm::Readable;

use super::{Compile, Compiler, Opcode, ReadableGuard, WritableGuard};

impl Compile for ast::Code<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        compiler.enter(
            self.span(),
            false,
            output.as_ref().map(|w| w.as_writable()),
            false,
            |compiler, display| {
                let join_output = output.is_some().then(|| WritableGuard::Joined);

                for expr in self.exprs() {
                    // Handle set rules specially.
                    if let ast::Expr::Set(set) = expr {
                        *display = true;
                        set.compile_into(engine, compiler, join_output.clone())?;
                        continue;
                    }

                    // Handle show rules specially.
                    if let ast::Expr::Show(show) = expr {
                        *display = true;
                        show.compile_into(engine, compiler, join_output.clone())?;
                        continue;
                    }

                    // Compile the expression, appending its output to the join
                    // output.
                    expr.compile_into(engine, compiler, join_output.clone())?;
                }

                Ok(())
            },
        )
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

        // Compile into the register.
        let output = Some(WritableGuard::from(reg.clone()));
        self.compile_into(engine, compiler, output)?;

        // Return the register.
        Ok(ReadableGuard::from(reg))
    }
}

impl Compile for ast::Expr<'_> {
    type Output = Option<WritableGuard>;

    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let span = self.span();
        let forbidden = |name: &str| {
            error!(span, "{} is only allowed directly in code and content blocks", name)
        };

        match self {
            ast::Expr::Text(text) => text.compile_into(engine, compiler, output),
            ast::Expr::Space(space) => space.compile_into(engine, compiler, output),
            ast::Expr::Linebreak(linebreak) => {
                linebreak.compile_into(engine, compiler, output)
            }
            ast::Expr::Parbreak(parbreak) => {
                parbreak.compile_into(engine, compiler, output)
            }
            ast::Expr::Escape(escape) => escape.compile_into(engine, compiler, output),
            ast::Expr::Shorthand(shorthand) => {
                shorthand.compile_into(engine, compiler, output)
            }
            ast::Expr::SmartQuote(smart_quote) => {
                smart_quote.compile_into(engine, compiler, output)
            }
            ast::Expr::Strong(strong) => strong.compile_into(engine, compiler, output),
            ast::Expr::Emph(emph) => emph.compile_into(engine, compiler, output),
            ast::Expr::Raw(raw) => raw.compile_into(engine, compiler, output),
            ast::Expr::Link(link) => link.compile_into(engine, compiler, output),
            ast::Expr::Label(label) => label.compile_into(engine, compiler, output),
            ast::Expr::Ref(ref_) => ref_.compile_into(engine, compiler, output),
            ast::Expr::Heading(heading) => heading.compile_into(engine, compiler, output),
            ast::Expr::List(list) => list.compile_into(engine, compiler, output),
            ast::Expr::Enum(enum_) => enum_.compile_into(engine, compiler, output),
            ast::Expr::Term(term) => term.compile_into(engine, compiler, output),
            ast::Expr::Equation(equation) => {
                equation.compile_into(engine, compiler, output)
            }
            ast::Expr::Math(math) => math.compile_into(engine, compiler, output),
            ast::Expr::MathIdent(math_ident) => {
                math_ident.compile_into(engine, compiler, output)
            }
            ast::Expr::MathAlignPoint(math_align_point) => {
                math_align_point.compile_into(engine, compiler, output)
            }
            ast::Expr::MathDelimited(math_delimited) => {
                math_delimited.compile_into(engine, compiler, output)
            }
            ast::Expr::MathAttach(math_attach) => {
                math_attach.compile_into(engine, compiler, output)
            }
            ast::Expr::MathPrimes(math_primes) => {
                math_primes.compile_into(engine, compiler, output)
            }
            ast::Expr::MathFrac(math_frac) => {
                math_frac.compile_into(engine, compiler, output)
            }
            ast::Expr::MathRoot(math_root) => {
                math_root.compile_into(engine, compiler, output)
            }
            ast::Expr::Ident(ident) => ident.compile_into(engine, compiler, output),
            ast::Expr::None(none_) => none_.compile_into(engine, compiler, output),
            ast::Expr::Auto(auto) => auto.compile_into(engine, compiler, output),
            ast::Expr::Bool(bool_) => bool_.compile_into(engine, compiler, output),
            ast::Expr::Int(int_) => int_.compile_into(engine, compiler, output),
            ast::Expr::Float(float_) => float_.compile_into(engine, compiler, output),
            ast::Expr::Numeric(numeric_) => {
                numeric_.compile_into(engine, compiler, output)
            }
            ast::Expr::Str(str_) => str_.compile_into(engine, compiler, output),
            ast::Expr::Code(code_) => code_.compile_into(engine, compiler, output),
            ast::Expr::Content(content_) => {
                content_.compile_into(engine, compiler, output)
            }
            ast::Expr::Parenthesized(parenthesized_) => {
                parenthesized_.compile_into(engine, compiler, output)
            }
            ast::Expr::Array(array_) => array_.compile_into(engine, compiler, output),
            ast::Expr::Dict(dict_) => dict_.compile_into(engine, compiler, output),
            ast::Expr::Unary(unary) => unary.compile_into(engine, compiler, output),
            ast::Expr::Binary(binary) => binary.compile_into(engine, compiler, output),
            ast::Expr::FieldAccess(field) => field.compile_into(engine, compiler, output),
            ast::Expr::FuncCall(call) => call.compile_into(engine, compiler, output),
            ast::Expr::Closure(closure) => closure.compile_into(engine, compiler, output),
            ast::Expr::Let(let_) => let_.compile_into(engine, compiler, output),
            ast::Expr::DestructAssign(destructure) => {
                destructure.compile_into(engine, compiler, output)
            }
            ast::Expr::Set(_) => bail!(forbidden("set")),
            ast::Expr::Show(_) => bail!(forbidden("show")),
            ast::Expr::Conditional(if_) => if_.compile_into(engine, compiler, output),
            ast::Expr::While(while_) => while_.compile_into(engine, compiler, output),
            ast::Expr::For(for_) => for_.compile_into(engine, compiler, output),
            ast::Expr::Break(break_) => break_.compile_into(engine, compiler, ()),
            ast::Expr::Continue(continue_) => {
                continue_.compile_into(engine, compiler, ())
            }
            ast::Expr::Return(return_) => return_.compile_into(engine, compiler, ()),
            ast::Expr::Import(import) => import.compile_into(engine, compiler, ()),
            ast::Expr::Include(include) => include.compile_into(engine, compiler, output),
        }
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let span = self.span();
        let forbidden = |name: &str| {
            error!(span, "{} is only allowed directly in code and content blocks", name)
        };

        match self {
            ast::Expr::Text(text) => text.compile(engine, compiler),
            ast::Expr::Space(space) => space.compile(engine, compiler),
            ast::Expr::Linebreak(linebreak) => linebreak.compile(engine, compiler),
            ast::Expr::Parbreak(parbreak) => parbreak.compile(engine, compiler),
            ast::Expr::Escape(escape) => escape.compile(engine, compiler),
            ast::Expr::Shorthand(shorthand) => shorthand.compile(engine, compiler),
            ast::Expr::SmartQuote(smart_quote) => smart_quote.compile(engine, compiler),
            ast::Expr::Strong(strong) => strong.compile(engine, compiler),
            ast::Expr::Emph(emph) => emph.compile(engine, compiler),
            ast::Expr::Raw(raw) => {
                raw.compile(engine, compiler).map(ReadableGuard::Constant)
            }
            ast::Expr::Link(link) => {
                link.compile(engine, compiler).map(ReadableGuard::Constant)
            }
            ast::Expr::Label(label) => {
                label.compile(engine, compiler).map(ReadableGuard::Constant)
            }
            ast::Expr::Ref(ref_) => ref_.compile(engine, compiler),
            ast::Expr::Heading(heading) => heading.compile(engine, compiler),
            ast::Expr::List(list) => list.compile(engine, compiler),
            ast::Expr::Enum(enum_) => enum_.compile(engine, compiler),
            ast::Expr::Term(term) => term.compile(engine, compiler),
            ast::Expr::Equation(equation) => equation.compile(engine, compiler),
            ast::Expr::Math(math) => math.compile(engine, compiler),
            ast::Expr::MathIdent(math_ident) => math_ident.compile(engine, compiler),
            ast::Expr::MathAlignPoint(math_align_point) => math_align_point
                .compile(engine, compiler)
                .map(ReadableGuard::Constant),
            ast::Expr::MathDelimited(math_delimited) => {
                math_delimited.compile(engine, compiler)
            }
            ast::Expr::MathAttach(math_attach) => math_attach.compile(engine, compiler),
            ast::Expr::MathPrimes(math_primes) => {
                math_primes.compile(engine, compiler).map(ReadableGuard::Constant)
            }
            ast::Expr::MathFrac(math_frac) => math_frac.compile(engine, compiler),
            ast::Expr::MathRoot(math_root) => math_root.compile(engine, compiler),
            ast::Expr::Ident(ident) => ident.compile(engine, compiler),
            ast::Expr::None(none_) => none_.compile(engine, compiler),
            ast::Expr::Auto(auto_) => auto_.compile(engine, compiler),
            ast::Expr::Bool(bool_) => bool_.compile(engine, compiler),
            ast::Expr::Int(int_) => int_.compile(engine, compiler),
            ast::Expr::Float(float_) => float_.compile(engine, compiler),
            ast::Expr::Numeric(numeric_) => numeric_.compile(engine, compiler),
            ast::Expr::Str(str_) => str_.compile(engine, compiler),
            ast::Expr::Code(code_) => code_.compile(engine, compiler),
            ast::Expr::Content(content_) => content_.compile(engine, compiler),
            ast::Expr::Parenthesized(parenthesized_) => {
                parenthesized_.compile(engine, compiler)
            }
            ast::Expr::Array(array_) => array_.compile(engine, compiler),
            ast::Expr::Dict(dict_) => dict_.compile(engine, compiler),
            ast::Expr::Unary(unary) => unary.compile(engine, compiler),
            ast::Expr::Binary(binary) => binary
                .compile(engine, compiler)
                .map(|r| r.unwrap_or(ReadableGuard::None)),
            ast::Expr::FieldAccess(field) => field.compile(engine, compiler),
            ast::Expr::FuncCall(call) => call.compile(engine, compiler),
            ast::Expr::Closure(closure) => closure.compile(engine, compiler),
            ast::Expr::Let(let_) => {
                let_.compile(engine, compiler).map(|_| ReadableGuard::None)
            }
            ast::Expr::DestructAssign(destructure) => {
                destructure.compile(engine, compiler).map(|_| ReadableGuard::None)
            }
            ast::Expr::Set(_) => bail!(forbidden("set")),
            ast::Expr::Show(_) => bail!(forbidden("show")),
            ast::Expr::Conditional(if_) => if_.compile(engine, compiler),
            ast::Expr::While(while_) => while_.compile(engine, compiler),
            ast::Expr::For(for_) => for_.compile(engine, compiler),
            ast::Expr::Break(break_) => {
                break_.compile(engine, compiler)?;
                Ok(ReadableGuard::None)
            }
            ast::Expr::Continue(continue_) => {
                continue_.compile(engine, compiler)?;
                Ok(ReadableGuard::None)
            }
            ast::Expr::Return(return_) => {
                return_.compile(engine, compiler)?;
                Ok(ReadableGuard::None)
            }
            ast::Expr::Import(import) => {
                import.compile(engine, compiler)?;
                Ok(ReadableGuard::None)
            }
            ast::Expr::Include(include) => include.compile(engine, compiler),
        }
    }
}

impl Compile for ast::Ident<'_> {
    type Output = Option<WritableGuard>;

    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // If we don't have an output, we do nothing.
        let Some(output) = output else {
            return Ok(());
        };

        let read = self.compile(engine, compiler)?;

        compiler.isr(Opcode::copy(self.span(), &read, &output));

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let Some(value) = compiler.read(self.span(), self.get()).at(self.span())? else {
            bail!(self.span(), "unknown variable: {}", self.get())
        };

        Ok(value)
    }
}

impl Compile for ast::None<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            compiler.isr(Opcode::none(self.span(), &output));
        }
        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        _: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        Ok(ReadableGuard::None)
    }
}

impl Compile for ast::Auto<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            compiler.isr(Opcode::auto(self.span(), &output));
        }
        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        _: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        Ok(ReadableGuard::Auto)
    }
}

impl Compile for ast::Bool<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            compiler.isr(Opcode::copy(self.span(), Readable::bool(self.get()), &output));
        }
        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        _: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        Ok(ReadableGuard::Bool(self.get()))
    }
}

impl Compile for ast::Int<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            let cst = self.compile(engine, compiler)?;
            compiler.isr(Opcode::copy(self.span(), &cst, &output));
        }

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let cst = compiler.const_(self.get());
        Ok(cst.into())
    }
}

impl Compile for ast::Float<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            let cst = self.compile(engine, compiler)?;
            compiler.isr(Opcode::copy(self.span(), &cst, &output));
        }

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let cst = compiler.const_(self.get());
        Ok(cst.into())
    }
}

impl Compile for ast::Numeric<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            let cst = self.compile(engine, compiler)?;
            compiler.isr(Opcode::copy(self.span(), &cst, &output));
        }

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let cst = compiler.const_(Value::numeric(self.get()));
        Ok(cst.into())
    }
}

impl Compile for ast::Str<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        if let Some(output) = output {
            let str = self.compile(engine, compiler)?;
            compiler.isr(Opcode::copy(self.span(), &str, &output));
        }

        Ok(())
    }

    fn compile(
        &self,
        _: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let str = compiler.string(self.get());
        Ok(str.into())
    }
}

impl Compile for ast::Array<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let Some(output) = output else {
            return Ok(());
        };

        if output.is_joined() {
            let input = self.compile(engine, compiler)?;
            compiler.isr(Opcode::copy(self.span(), &input, &output));

            return Ok(());
        }

        let cap = self.items().count();
        compiler.isr(Opcode::array(self.span(), cap as u32, &output));

        for item in self.items() {
            match item {
                ast::ArrayItem::Pos(item) => {
                    let value = item.compile(engine, compiler)?;
                    compiler.isr(Opcode::push(item.span(), &value, &output));
                }
                ast::ArrayItem::Spread(item) => {
                    let value = item.compile(engine, compiler)?;
                    compiler.isr(Opcode::spread(item.span(), &value, &output));
                }
            }
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}

impl Compile for ast::Dict<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let Some(output) = output else {
            return Ok(());
        };

        if output.is_joined() {
            let input = self.compile(engine, compiler)?;
            compiler.isr(Opcode::copy(self.span(), &input, &output));

            return Ok(());
        }

        let cap = self.items().count();
        compiler.isr(Opcode::dict(self.span(), cap as u32, &output));

        for item in self.items() {
            match item {
                ast::DictItem::Named(item) => {
                    let key = compiler.string(item.name().get().clone());
                    let value = item.expr().compile(engine, compiler)?;
                    compiler.isr(Opcode::insert(item.span(), key, &value, &output));
                }
                ast::DictItem::Keyed(item) => {
                    let key = item.key().compile(engine, compiler)?;
                    let value = item.expr().compile(engine, compiler)?;
                    compiler.isr(Opcode::insert(item.span(), &key, &value, &output));
                }
                ast::DictItem::Spread(item) => {
                    let value = item.compile(engine, compiler)?;
                    compiler.isr(Opcode::spread(item.span(), &value, &output));
                }
            }
        }

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        let output = compiler.register().at(self.span())?;
        self.compile_into(engine, compiler, Some(output.clone().into()))?;
        Ok(output.into())
    }
}

impl Compile for ast::CodeBlock<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // We do not create a scope because `ast::Code` already creates one
        self.body().compile_into(engine, compiler, output)
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // We do not create a scope because `ast::Code` already creates one
        self.body().compile(engine, compiler)
    }
}

impl Compile for ast::ContentBlock<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        // We do not create a scope because `ast::Content` already creates one
        self.body().compile_into(engine, compiler, output)
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // We do not create a scope because `ast::Content` already creates one
        self.body().compile(engine, compiler)
    }
}

impl Compile for ast::Parenthesized<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        self.expr().compile_into(engine, compiler, output)
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        self.expr().compile(engine, compiler)
    }
}

impl Compile for ast::FieldAccess<'_> {
    type Output = Option<WritableGuard>;
    type IntoOutput = ReadableGuard;

    fn compile_into(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
        output: Self::Output,
    ) -> SourceResult<()> {
        let Some(output) = output else {
            return Ok(());
        };

        let pattern = self.target().access(engine, compiler, false)?;

        let access =
            AccessPattern::Chained(Arc::new(pattern), self.field().get().clone());
        let access_id = compiler.access(access.as_vm_access());

        compiler.isr(Opcode::field(self.span(), access_id, &output));

        Ok(())
    }

    fn compile(
        &self,
        engine: &mut Engine,
        compiler: &mut Compiler,
    ) -> SourceResult<Self::IntoOutput> {
        // Get an output register.
        let reg = compiler.register().at(self.span())?;

        // Compile into the register.
        self.compile_into(engine, compiler, Some(reg.clone().into()))?;

        // Return the register.
        Ok(reg.into())
    }
}

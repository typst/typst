use typst_syntax::ast::{self, AstNode};
use typst_utils::PicoStr;

use crate::diag::{bail, error, SourceResult};
use crate::engine::Engine;
use crate::foundations::Value;
use crate::lang::compiled::CompiledClosure;
use crate::lang::compiler::{Access, CompileAccess};
use crate::lang::operands::Readable;

use super::{Compile, CompileTopLevel, Compiler, ReadableGuard, WritableGuard};

impl CompileTopLevel for ast::Code<'_> {
    fn compile_top_level<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
    ) -> SourceResult<()> {
        for expr in self.exprs() {
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

impl Compile for ast::Code<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        compiler.enter(engine, self.span(), output, |compiler, engine| {
            let mut is_content = false;
            for expr in self.exprs() {
                // Handle set rules specially.
                if let ast::Expr::Set(set) = expr {
                    is_content = true;
                    set.compile(compiler, engine, WritableGuard::Joined)?;
                    compiler.flow();
                    continue;
                }

                // Handle show rules specially.
                if let ast::Expr::Show(show) = expr {
                    is_content = true;
                    show.compile(compiler, engine, WritableGuard::Joined)?;
                    compiler.flow();
                    continue;
                }

                // Compile the expression, appending its output to the join
                // output.
                expr.compile(compiler, engine, WritableGuard::Joined)?;
                compiler.flow();
            }

            Ok(is_content)
        })
    }
}

impl Compile for ast::Expr<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let span = self.span();
        let forbidden = |name: &str| {
            error!(span, "{} is only allowed directly in code and content blocks", name)
        };

        match self {
            ast::Expr::Text(text) => text.compile(compiler, engine, output),
            ast::Expr::Space(space) => space.compile(compiler, engine, output),
            ast::Expr::Linebreak(linebreak) => {
                linebreak.compile(compiler, engine, output)
            }
            ast::Expr::Parbreak(parbreak) => parbreak.compile(compiler, engine, output),
            ast::Expr::Escape(escape) => escape.compile(compiler, engine, output),
            ast::Expr::Shorthand(shorthand) => {
                shorthand.compile(compiler, engine, output)
            }
            ast::Expr::SmartQuote(smart_quote) => {
                smart_quote.compile(compiler, engine, output)
            }
            ast::Expr::Strong(strong) => strong.compile(compiler, engine, output),
            ast::Expr::Emph(emph) => emph.compile(compiler, engine, output),
            ast::Expr::Raw(raw) => raw.compile(compiler, engine, output),
            ast::Expr::Link(link) => link.compile(compiler, engine, output),
            ast::Expr::Label(label) => label.compile(compiler, engine, output),
            ast::Expr::Ref(ref_) => ref_.compile(compiler, engine, output),
            ast::Expr::Heading(heading) => heading.compile(compiler, engine, output),
            ast::Expr::List(list) => list.compile(compiler, engine, output),
            ast::Expr::Enum(enum_) => enum_.compile(compiler, engine, output),
            ast::Expr::Term(term) => term.compile(compiler, engine, output),
            ast::Expr::Equation(equation) => equation.compile(compiler, engine, output),
            ast::Expr::Math(math) => math.compile(compiler, engine, output),
            ast::Expr::MathIdent(math_ident) => {
                math_ident.compile(compiler, engine, output)
            }
            ast::Expr::MathAlignPoint(math_align_point) => {
                math_align_point.compile(compiler, engine, output)
            }
            ast::Expr::MathDelimited(math_delimited) => {
                math_delimited.compile(compiler, engine, output)
            }
            ast::Expr::MathAttach(math_attach) => {
                math_attach.compile(compiler, engine, output)
            }
            ast::Expr::MathPrimes(math_primes) => {
                math_primes.compile(compiler, engine, output)
            }
            ast::Expr::MathFrac(math_frac) => math_frac.compile(compiler, engine, output),
            ast::Expr::MathRoot(math_root) => math_root.compile(compiler, engine, output),
            ast::Expr::Ident(ident) => ident.compile(compiler, engine, output),
            ast::Expr::None(none_) => none_.compile(compiler, engine, output),
            ast::Expr::Auto(auto) => auto.compile(compiler, engine, output),
            ast::Expr::Bool(bool_) => bool_.compile(compiler, engine, output),
            ast::Expr::Int(int_) => int_.compile(compiler, engine, output),
            ast::Expr::Float(float_) => float_.compile(compiler, engine, output),
            ast::Expr::Numeric(numeric_) => numeric_.compile(compiler, engine, output),
            ast::Expr::Str(str_) => str_.compile(compiler, engine, output),
            ast::Expr::Code(code_) => code_.compile(compiler, engine, output),
            ast::Expr::Content(content_) => content_.compile(compiler, engine, output),
            ast::Expr::Parenthesized(parenthesized_) => {
                parenthesized_.compile(compiler, engine, output)
            }
            ast::Expr::Array(array_) => array_.compile(compiler, engine, output),
            ast::Expr::Dict(dict_) => dict_.compile(compiler, engine, output),
            ast::Expr::Unary(unary) => unary.compile(compiler, engine, output),
            ast::Expr::Binary(binary) => binary.compile(compiler, engine, output),
            ast::Expr::FieldAccess(field) => field.compile(compiler, engine, output),
            ast::Expr::FuncCall(call) => call.compile(compiler, engine, output),
            ast::Expr::Closure(closure) => closure.compile(compiler, engine, output),
            ast::Expr::Let(let_) => {
                let_.compile(compiler, engine, WritableGuard::Joined)?;
                if !output.is_joiner() {
                    compiler.copy(span, Readable::None, output)
                }

                Ok(())
            }
            ast::Expr::DestructAssign(destructure) => {
                destructure.compile(compiler, engine, WritableGuard::Joined)?;
                if !output.is_joiner() {
                    compiler.copy(span, Readable::None, output)
                }

                Ok(())
            }
            ast::Expr::Set(_) => bail!(forbidden("set")),
            ast::Expr::Show(_) => bail!(forbidden("show")),
            ast::Expr::Conditional(if_) => if_.compile(compiler, engine, output),
            ast::Expr::While(while_) => while_.compile(compiler, engine, output),
            ast::Expr::For(for_) => for_.compile(compiler, engine, output),
            ast::Expr::Break(break_) => break_.compile(compiler, engine, output),
            ast::Expr::Continue(continue_) => continue_.compile(compiler, engine, output),
            ast::Expr::Return(return_) => return_.compile(compiler, engine, output),
            ast::Expr::Import(import) => import.compile(compiler, engine, output),
            ast::Expr::Include(include) => include.compile(compiler, engine, output),
            ast::Expr::Contextual(contextual) => {
                contextual.compile(compiler, engine, output)
            }
        }
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let span = self.span();
        let forbidden = |name: &str| {
            error!(span, "{} is only allowed directly in code and content blocks", name)
        };

        match self {
            ast::Expr::Text(text) => text.compile_to_readable(compiler, engine),
            ast::Expr::Space(space) => space.compile_to_readable(compiler, engine),
            ast::Expr::Linebreak(linebreak) => {
                linebreak.compile_to_readable(compiler, engine)
            }
            ast::Expr::Parbreak(parbreak) => {
                parbreak.compile_to_readable(compiler, engine)
            }
            ast::Expr::Escape(escape) => escape.compile_to_readable(compiler, engine),
            ast::Expr::Shorthand(shorthand) => {
                shorthand.compile_to_readable(compiler, engine)
            }
            ast::Expr::SmartQuote(smart_quote) => {
                smart_quote.compile_to_readable(compiler, engine)
            }
            ast::Expr::Strong(strong) => strong.compile_to_readable(compiler, engine),
            ast::Expr::Emph(emph) => emph.compile_to_readable(compiler, engine),
            ast::Expr::Raw(raw) => raw.compile_to_readable(compiler, engine),
            ast::Expr::Link(link) => link.compile_to_readable(compiler, engine),
            ast::Expr::Label(label) => label.compile_to_readable(compiler, engine),
            ast::Expr::Ref(ref_) => ref_.compile_to_readable(compiler, engine),
            ast::Expr::Heading(heading) => heading.compile_to_readable(compiler, engine),
            ast::Expr::List(list) => list.compile_to_readable(compiler, engine),
            ast::Expr::Enum(enum_) => enum_.compile_to_readable(compiler, engine),
            ast::Expr::Term(term) => term.compile_to_readable(compiler, engine),
            ast::Expr::Equation(equation) => {
                equation.compile_to_readable(compiler, engine)
            }
            ast::Expr::Math(math) => math.compile_to_readable(compiler, engine),
            ast::Expr::MathIdent(math_ident) => {
                math_ident.compile_to_readable(compiler, engine)
            }
            ast::Expr::MathAlignPoint(math_align_point) => {
                math_align_point.compile_to_readable(compiler, engine)
            }
            ast::Expr::MathDelimited(math_delimited) => {
                math_delimited.compile_to_readable(compiler, engine)
            }
            ast::Expr::MathAttach(math_attach) => {
                math_attach.compile_to_readable(compiler, engine)
            }
            ast::Expr::MathPrimes(math_primes) => {
                math_primes.compile_to_readable(compiler, engine)
            }
            ast::Expr::MathFrac(math_frac) => {
                math_frac.compile_to_readable(compiler, engine)
            }
            ast::Expr::MathRoot(math_root) => {
                math_root.compile_to_readable(compiler, engine)
            }
            ast::Expr::Ident(ident) => ident.compile_to_readable(compiler, engine),
            ast::Expr::None(none_) => none_.compile_to_readable(compiler, engine),
            ast::Expr::Auto(auto) => auto.compile_to_readable(compiler, engine),
            ast::Expr::Bool(bool_) => bool_.compile_to_readable(compiler, engine),
            ast::Expr::Int(int_) => int_.compile_to_readable(compiler, engine),
            ast::Expr::Float(float_) => float_.compile_to_readable(compiler, engine),
            ast::Expr::Numeric(numeric_) => {
                numeric_.compile_to_readable(compiler, engine)
            }
            ast::Expr::Str(str_) => str_.compile_to_readable(compiler, engine),
            ast::Expr::Code(code_) => code_.compile_to_readable(compiler, engine),
            ast::Expr::Content(content_) => {
                content_.compile_to_readable(compiler, engine)
            }
            ast::Expr::Parenthesized(parenthesized_) => {
                parenthesized_.compile_to_readable(compiler, engine)
            }
            ast::Expr::Array(array_) => array_.compile_to_readable(compiler, engine),
            ast::Expr::Dict(dict_) => dict_.compile_to_readable(compiler, engine),
            ast::Expr::Unary(unary) => unary.compile_to_readable(compiler, engine),
            ast::Expr::Binary(binary) => binary.compile_to_readable(compiler, engine),
            ast::Expr::FieldAccess(field) => field.compile_to_readable(compiler, engine),
            ast::Expr::FuncCall(call) => call.compile_to_readable(compiler, engine),
            ast::Expr::Closure(closure) => closure.compile_to_readable(compiler, engine),
            ast::Expr::Let(let_) => let_.compile_to_readable(compiler, engine),
            ast::Expr::DestructAssign(destructure) => {
                destructure.compile_to_readable(compiler, engine)
            }
            ast::Expr::Set(_) => bail!(forbidden("set")),
            ast::Expr::Show(_) => bail!(forbidden("show")),
            ast::Expr::Conditional(if_) => if_.compile_to_readable(compiler, engine),
            ast::Expr::While(while_) => while_.compile_to_readable(compiler, engine),
            ast::Expr::For(for_) => for_.compile_to_readable(compiler, engine),
            ast::Expr::Break(break_) => break_.compile_to_readable(compiler, engine),
            ast::Expr::Continue(continue_) => {
                continue_.compile_to_readable(compiler, engine)
            }
            ast::Expr::Return(return_) => return_.compile_to_readable(compiler, engine),
            ast::Expr::Import(import) => import.compile_to_readable(compiler, engine),
            ast::Expr::Include(include) => include.compile_to_readable(compiler, engine),
            ast::Expr::Contextual(contextual) => {
                contextual.compile_to_readable(compiler, engine)
            }
        }
    }
}

impl Compile for ast::Ident<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let read = self.compile_to_readable(compiler, engine)?;

        compiler.copy(self.span(), read, output);

        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let Some(value) = compiler.read(self.span(), self.get(), false) else {
            bail!(self.span(), "unknown variable: {}", self.get())
        };

        Ok(value)
    }
}

impl Compile for ast::None<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        compiler.none(self.span(), output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        _: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        Ok(ReadableGuard::None)
    }
}

impl Compile for ast::Auto<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        compiler.auto(self.span(), output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        _: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        Ok(ReadableGuard::Auto)
    }
}

impl Compile for ast::Bool<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        compiler.copy(self.span(), Readable::Bool(self.get()), output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        _: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        Ok(ReadableGuard::Bool(self.get()))
    }
}

impl Compile for ast::Int<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let cst = self.compile_to_readable(compiler, engine)?;
        compiler.copy(self.span(), cst, output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let cst = compiler.const_(self.get());
        Ok(cst.into())
    }
}

impl Compile for ast::Float<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let cst = self.compile_to_readable(compiler, engine)?;
        compiler.copy(self.span(), cst, output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let cst = compiler.const_(self.get());
        Ok(cst.into())
    }
}

impl Compile for ast::Numeric<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let cst = self.compile_to_readable(compiler, engine)?;
        compiler.copy(self.span(), cst, output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let cst = compiler.const_(Value::numeric(self.get()));
        Ok(cst.into())
    }
}

impl Compile for ast::Str<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let str = self.compile_to_readable(compiler, engine)?;
        compiler.copy(self.span(), str, output);
        Ok(())
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        _: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        let str = compiler.string(self.get());
        Ok(str.into())
    }
}

impl Compile for ast::Array<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        if output.is_joiner() {
            let input = self.compile_to_readable(compiler, engine)?;
            compiler.copy(self.span(), input, output);

            return Ok(());
        }

        let cap = self.items().count();
        compiler.array(self.span(), cap as u32, output.clone());

        for item in self.items() {
            match item {
                ast::ArrayItem::Pos(item) => {
                    let value = item.compile_to_readable(compiler, engine)?;
                    compiler.push(item.span(), value, output.clone());
                }
                ast::ArrayItem::Spread(item) => {
                    let value = item.expr().compile_to_readable(compiler, engine)?;
                    compiler.spread(item.span(), value, output.clone());
                }
            }
        }

        Ok(())
    }
}

impl Compile for ast::Dict<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        if output.is_joiner() {
            let input = self.compile_to_readable(compiler, engine)?;
            compiler.copy(self.span(), input, output);

            return Ok(());
        }

        let cap = self.items().count();
        compiler.dict(self.span(), cap as u32, output.clone());

        for item in self.items() {
            match item {
                ast::DictItem::Named(item) => {
                    let key = compiler.string(item.name().get().clone());
                    let value = item.expr().compile_to_readable(compiler, engine)?;
                    compiler.insert(item.span(), key, value, output.clone());
                }
                ast::DictItem::Keyed(item) => {
                    let key = item.key().compile_to_readable(compiler, engine)?;
                    let value = item.expr().compile_to_readable(compiler, engine)?;
                    compiler.insert(item.span(), key, value, output.clone());
                }
                ast::DictItem::Spread(spread) => {
                    let value = spread.expr().compile_to_readable(compiler, engine)?;
                    compiler.spread(spread.span(), value, output.clone());
                }
            }
        }

        Ok(())
    }
}

impl Compile for ast::CodeBlock<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        self.body().compile(compiler, engine, output)
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        self.body().compile_to_readable(compiler, engine)
    }
}
impl Compile for ast::ContentBlock<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        self.body().compile(compiler, engine, output)
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        self.body().compile_to_readable(compiler, engine)
    }
}

impl Compile for ast::Parenthesized<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        self.expr().compile(compiler, engine, output)
    }

    fn compile_to_readable<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
    ) -> SourceResult<ReadableGuard> {
        self.expr().compile_to_readable(compiler, engine)
    }
}

impl Compile for ast::FieldAccess<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        let pattern = self.target().access(compiler, engine, false)?;
        let index = compiler.access(pattern);

        let access =
            Access::Chained(self.span(), index, PicoStr::new(self.field().get()));

        // If we can resolve the access to a constant, we can copy it directly.
        if let Some(value) = access.resolve(compiler)? {
            let const_id = compiler.const_(value);
            compiler.copy(self.span(), const_id, output);

            return Ok(());
        }

        // Otherwise we need to field the access.
        let access_id = compiler.access(access);
        compiler.field(self.span(), access_id, output);

        Ok(())
    }
}

impl Compile for ast::Contextual<'_> {
    fn compile<'lib>(
        &self,
        compiler: &mut Compiler<'lib>,
        engine: &mut Engine,
        output: WritableGuard,
    ) -> SourceResult<()> {
        // Compile the contextual as if it was a closure.
        // Since it doesn't have any arguments, we don't need to do any
        // processing of arguments and default values.
        let mut closure_compiler =
            Compiler::new_closure(compiler, PicoStr::from("contextual"));

        // Compile the body of the contextual.
        match self.body() {
            ast::Expr::Code(code) => {
                code.body().compile_top_level(&mut closure_compiler, engine)?;
            }
            ast::Expr::Content(content) => {
                content.body().compile_top_level(&mut closure_compiler, engine)?;
            }
            other => {
                other.compile(&mut closure_compiler, engine, WritableGuard::Joined)?
            }
        }

        // Ensure that a flow event is present.
        closure_compiler.flow();

        // Collect the compiled closure.
        let closure = closure_compiler.finish_closure(self.span(), vec![], None)?;

        // Get the closure ID.
        let compiled = CompiledClosure::new(closure, &*compiler);
        let closure_id = compiler.closure(compiled);

        // Instantiate the closure.
        compiler.instantiate(self.span(), closure_id, output.clone());

        // Create the contextual element
        compiler.contextual(self.span(), output.clone(), output);

        Ok(())
    }
}

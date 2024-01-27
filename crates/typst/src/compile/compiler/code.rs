use ecow::eco_vec;
use typst_syntax::ast::{self, AstNode};

use crate::compile::RegisterOrString;
use crate::diag::{bail, error, At, SourceResult};
use crate::foundations::Value;

use super::{Compile, Compiler, Instruction, Register};

impl Compile for ast::Code<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        // In order to keep track of styles, every time we encounter a row, we
        // push a new style context onto the stack. This context is popped when
        // we encounter a row end.
        let mut stack_depth = 0;

        let mut exprs = self.exprs();
        let size_hint = exprs.size_hint().1.unwrap_or_else(|| exprs.size_hint().0);
        let output = compiler.reg().at(self.span())?;

        // We want to start joining values so we push a new
        // join group.
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::JoinGroup { content: false, capacity: size_hint as u16 });

        for expr in &mut exprs {
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

                compiler.free(style);
                compiler.instructions.push(Instruction::StylePush { style });

                continue;
            }

            let one = expr.compile(compiler)?;
            match one {
                Register::NONE => {}
                one => {
                    // We already have a join group, so we just join the value.
                    compiler.spans.push(expr.span());
                    compiler.instructions.push(Instruction::Join { value: one });
                    compiler.free(one);
                }
            }
        }

        if stack_depth > 0 {
            compiler.spans.push(self.span());
            compiler
                .instructions
                .push(Instruction::StylePop { depth: stack_depth });
        }

        // If we have a join group pop it and produce the output.
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::PopGroup { target: output, content: false });

        Ok(output)
    }
}

impl Compile for ast::Expr<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let span = self.span();
        let forbidden = |name: &str| {
            error!(span, "{} is only allowed directly in code and content blocks", name)
        };

        match self {
            ast::Expr::Text(txt) => txt.compile(compiler),
            ast::Expr::Space(space) => space.compile(compiler),
            ast::Expr::Linebreak(linebreak) => linebreak.compile(compiler),
            ast::Expr::Parbreak(parbreak) => parbreak.compile(compiler),
            ast::Expr::Escape(escape) => escape.compile(compiler),
            ast::Expr::Shorthand(shorthand) => shorthand.compile(compiler),
            ast::Expr::SmartQuote(smart_quote) => smart_quote.compile(compiler),
            ast::Expr::Strong(strong) => strong.compile(compiler),
            ast::Expr::Emph(emph) => emph.compile(compiler),
            ast::Expr::Raw(raw) => raw.compile(compiler),
            ast::Expr::Link(link) => link.compile(compiler),
            ast::Expr::Label(label) => label.compile(compiler),
            ast::Expr::Ref(ref_) => ref_.compile(compiler),
            ast::Expr::Heading(heading) => heading.compile(compiler),
            ast::Expr::List(list) => list.compile(compiler),
            ast::Expr::Enum(enum_) => enum_.compile(compiler),
            ast::Expr::Term(term) => term.compile(compiler),
            ast::Expr::Equation(equation) => equation.compile(compiler),
            ast::Expr::Math(math) => math.compile(compiler),
            ast::Expr::MathIdent(math_ident) => math_ident.compile(compiler),
            ast::Expr::MathAlignPoint(math_align_point) => {
                math_align_point.compile(compiler)
            }
            ast::Expr::MathDelimited(math_delimited) => math_delimited.compile(compiler),
            ast::Expr::MathAttach(math_attach) => math_attach.compile(compiler),
            ast::Expr::MathPrimes(math_primes) => math_primes.compile(compiler),
            ast::Expr::MathFrac(math_frac) => math_frac.compile(compiler),
            ast::Expr::MathRoot(math_root) => math_root.compile(compiler),
            ast::Expr::Ident(ident) => ident.compile(compiler),
            ast::Expr::None(none) => none.compile(compiler),
            ast::Expr::Auto(auto) => auto.compile(compiler),
            ast::Expr::Bool(bool_) => bool_.compile(compiler),
            ast::Expr::Int(int) => int.compile(compiler),
            ast::Expr::Float(float) => float.compile(compiler),
            ast::Expr::Numeric(numeric) => numeric.compile(compiler),
            ast::Expr::Str(str_) => str_.compile(compiler),
            ast::Expr::Code(code) => code.compile(compiler),
            ast::Expr::Content(content) => content.compile(compiler),
            ast::Expr::Parenthesized(parenthesized) => parenthesized.compile(compiler),
            ast::Expr::Array(array) => array.compile(compiler),
            ast::Expr::Dict(dict) => dict.compile(compiler),
            ast::Expr::Unary(unary) => unary.compile(compiler),
            ast::Expr::Binary(binary) => binary.compile(compiler),
            ast::Expr::FieldAccess(field_access) => field_access.compile(compiler),
            ast::Expr::FuncCall(func_call) => func_call.compile(compiler),
            ast::Expr::Closure(closure) => closure.compile(compiler),
            ast::Expr::Let(let_) => let_.compile(compiler),
            ast::Expr::DestructAssign(destruct_assign) => {
                destruct_assign.compile(compiler)
            }
            ast::Expr::Set(_) => bail!(forbidden("set")),
            ast::Expr::Show(_) => bail!(forbidden("show")),
            ast::Expr::Conditional(cond) => cond.compile(compiler),
            ast::Expr::While(while_) => while_.compile(compiler),
            ast::Expr::For(for_) => for_.compile(compiler),
            ast::Expr::Import(import) => import.compile(compiler),
            ast::Expr::Include(include) => include.compile(compiler),
            ast::Expr::Break(break_) => break_.compile(compiler),
            ast::Expr::Continue(continue_) => continue_.compile(compiler),
            ast::Expr::Return(ret) => ret.compile(compiler),
        }
    }

    fn compile_display(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let span = self.span();
        let forbidden = |name: &str| {
            error!(span, "{} is only allowed directly in code and content blocks", name)
        };

        match self {
            ast::Expr::Text(txt) => txt.compile_display(compiler),
            ast::Expr::Space(space) => space.compile_display(compiler),
            ast::Expr::Linebreak(linebreak) => linebreak.compile_display(compiler),
            ast::Expr::Parbreak(parbreak) => parbreak.compile_display(compiler),
            ast::Expr::Escape(escape) => escape.compile_display(compiler),
            ast::Expr::Shorthand(shorthand) => shorthand.compile_display(compiler),
            ast::Expr::SmartQuote(smart_quote) => smart_quote.compile_display(compiler),
            ast::Expr::Strong(strong) => strong.compile_display(compiler),
            ast::Expr::Emph(emph) => emph.compile_display(compiler),
            ast::Expr::Raw(raw) => raw.compile_display(compiler),
            ast::Expr::Link(link) => link.compile_display(compiler),
            ast::Expr::Label(label) => label.compile_display(compiler),
            ast::Expr::Ref(ref_) => ref_.compile_display(compiler),
            ast::Expr::Heading(heading) => heading.compile_display(compiler),
            ast::Expr::List(list) => list.compile_display(compiler),
            ast::Expr::Enum(enum_) => enum_.compile_display(compiler),
            ast::Expr::Term(term) => term.compile_display(compiler),
            ast::Expr::Equation(equation) => equation.compile_display(compiler),
            ast::Expr::Math(math) => math.compile_display(compiler),
            ast::Expr::MathIdent(math_ident) => math_ident.compile_display(compiler),
            ast::Expr::MathAlignPoint(math_align_point) => {
                math_align_point.compile_display(compiler)
            }
            ast::Expr::MathDelimited(math_delimited) => {
                math_delimited.compile_display(compiler)
            }
            ast::Expr::MathAttach(math_attach) => math_attach.compile_display(compiler),
            ast::Expr::MathPrimes(math_primes) => math_primes.compile_display(compiler),
            ast::Expr::MathFrac(math_frac) => math_frac.compile_display(compiler),
            ast::Expr::MathRoot(math_root) => math_root.compile_display(compiler),
            ast::Expr::Ident(ident) => ident.compile_display(compiler),
            ast::Expr::None(none) => none.compile_display(compiler),
            ast::Expr::Auto(auto) => auto.compile_display(compiler),
            ast::Expr::Bool(bool_) => bool_.compile_display(compiler),
            ast::Expr::Int(int) => int.compile_display(compiler),
            ast::Expr::Float(float) => float.compile_display(compiler),
            ast::Expr::Numeric(numeric) => numeric.compile_display(compiler),
            ast::Expr::Str(str_) => str_.compile_display(compiler),
            ast::Expr::Code(code) => code.compile_display(compiler),
            ast::Expr::Content(content) => content.compile_display(compiler),
            ast::Expr::Parenthesized(parenthesized) => {
                parenthesized.compile_display(compiler)
            }
            ast::Expr::Array(array) => array.compile_display(compiler),
            ast::Expr::Dict(dict) => dict.compile_display(compiler),
            ast::Expr::Unary(unary) => unary.compile_display(compiler),
            ast::Expr::Binary(binary) => binary.compile_display(compiler),
            ast::Expr::FieldAccess(field_access) => {
                field_access.compile_display(compiler)
            }
            ast::Expr::FuncCall(func_call) => func_call.compile_display(compiler),
            ast::Expr::Closure(closure) => closure.compile_display(compiler),
            ast::Expr::Let(let_) => let_.compile_display(compiler),
            ast::Expr::DestructAssign(destruct_assign) => {
                destruct_assign.compile_display(compiler)
            }
            ast::Expr::Set(_) => bail!(forbidden("set")),
            ast::Expr::Show(_) => bail!(forbidden("show")),
            ast::Expr::Conditional(cond) => cond.compile_display(compiler),
            ast::Expr::While(while_) => while_.compile_display(compiler),
            ast::Expr::For(for_) => for_.compile_display(compiler),
            ast::Expr::Import(import) => import.compile_display(compiler),
            ast::Expr::Include(include) => include.compile_display(compiler),
            ast::Expr::Break(break_) => break_.compile_display(compiler),
            ast::Expr::Continue(continue_) => continue_.compile_display(compiler),
            ast::Expr::Return(ret) => ret.compile_display(compiler),
        }
    }
}

impl Compile for ast::Ident<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let isr = compiler.local_ref(self.get(), reg).ok_or_else(|| {
            eco_vec![error!(self.span(), "unknown identifier: `{}`", self.get())]
        })?;

        compiler.spans.push(self.span());
        compiler.instructions.push(isr);

        Ok(reg)
    }
}

impl Compile for ast::None<'_> {
    fn compile(&self, _: &mut Compiler) -> SourceResult<Register> {
        Ok(Register::NONE)
    }
}

impl Compile for ast::Auto<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = compiler.const_(Value::Auto);

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { register: reg, value });

        Ok(reg)
    }
}

impl Compile for ast::Bool<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = compiler.const_(Value::Bool(self.get()));

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { register: reg, value });

        Ok(reg)
    }
}

impl Compile for ast::Int<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = compiler.const_(Value::Int(self.get()));

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { register: reg, value });

        Ok(reg)
    }
}

impl Compile for ast::Float<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = compiler.const_(Value::Float(self.get()));

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { register: reg, value });

        Ok(reg)
    }
}

impl Compile for ast::Numeric<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = compiler.const_(Value::numeric(self.get()));

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { register: reg, value });

        Ok(reg)
    }
}

impl Compile for ast::Str<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = compiler.const_(Value::Str(self.get().into()));

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Set { register: reg, value });

        Ok(reg)
    }
}

impl Compile for ast::Array<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;

        let i = compiler.instructions.len();
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::Array { size: 0, target: reg });

        let mut count = 0;
        for item in self.items() {
            compiler.spans.push(item.span());
            match item {
                ast::ArrayItem::Pos(pos) => {
                    let value = pos.compile(compiler)?;
                    compiler
                        .instructions
                        .push(Instruction::ArrayPush { array: reg, value });
                    compiler.free(value);
                }
                ast::ArrayItem::Spread(spread) => {
                    let value = spread.compile(compiler)?;
                    compiler
                        .instructions
                        .push(Instruction::ArraySpread { array: reg, value });
                    compiler.free(value);
                }
            }
            count += 1;
        }

        compiler.instructions.make_mut()[i] =
            Instruction::Array { size: count, target: reg };

        Ok(reg)
    }
}

impl Compile for ast::Dict<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;

        let i = compiler.instructions.len();
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Dict { size: 0, target: reg });

        let mut count = 0;
        for item in self.items() {
            compiler.spans.push(item.span());
            match item {
                ast::DictItem::Named(named) => {
                    let value = named.expr().compile(compiler)?;
                    let string_id = compiler.string(named.name().get());

                    compiler.instructions.push(Instruction::DictInsert {
                        dict: reg,
                        key: RegisterOrString::String(string_id),
                        value,
                    });
                    compiler.free(value);
                }
                ast::DictItem::Keyed(keyed) => {
                    let key = keyed.key().compile(compiler)?;
                    let value = keyed.expr().compile(compiler)?;

                    compiler.instructions.push(Instruction::DictInsert {
                        dict: reg,
                        key: RegisterOrString::Register(key),
                        value,
                    });
                    compiler.free(key);
                    compiler.free(value);
                }
                ast::DictItem::Spread(spread) => {
                    let value = spread.compile(compiler)?;
                    compiler
                        .instructions
                        .push(Instruction::DictSpread { dict: reg, value });
                    compiler.free(value);
                }
            }
            count += 1;
        }

        compiler.instructions.make_mut()[i] =
            Instruction::Dict { size: count, target: reg };

        Ok(reg)
    }
}

impl Compile for ast::CodeBlock<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        compiler.in_scope(self.span(), |compiler| self.body().compile(compiler))
    }
}

impl Compile for ast::ContentBlock<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        compiler.in_scope(self.span(), |compiler| self.body().compile(compiler))
    }
}

impl Compile for ast::Parenthesized<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        self.expr().compile(compiler)
    }
}

impl Compile for ast::FieldAccess<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let reg = compiler.reg().at(self.span())?;
        let value = self.target().compile(compiler)?;
        let field = compiler.string(self.field().get());

        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::FieldAccess {
            value,
            field,
            target: reg,
        });

        compiler.free(value);

        Ok(reg)
    }
}

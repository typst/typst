use ecow::{EcoString, EcoVec};
use typst_syntax::{
    ast::{self, AstNode},
    Span,
};

use crate::{
    compile::{
        destructure::{Pattern, PatternCompile},
        ArgumentId, Call, CapturedId, Instruction, LocalId, PatternItem, PatternKind,
        Register, ScopeId,
    },
    diag::{At, SourceResult},
    foundations::{is_mutating_method, Label, Value},
    World,
};

use super::{Access, AccessPattern, Compile, Compiler};

impl Compile for ast::FuncCall<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        let span = self.span();
        let callee = self.callee();
        let in_math = in_math(callee);
        let args = self.args();
        let trailing_comma = args.trailing_comma();

        let args = args.compile(compiler)?;

        // Try to compile an associated function.
        let mut callee_reg = None;
        let callee = if let ast::Expr::FieldAccess(access) = callee {
            let field = access.field();

            // If this is a mutating method, we need to access the target instead
            // of the usual copy.
            if is_mutating_method(&field) {
                access.access(compiler, true)?
            } else {
                let c = self.callee().compile(compiler)?;
                callee_reg = Some(c);
                AccessPattern::Register(c)
            }
        } else {
            let c = self.callee().compile(compiler)?;
            callee_reg = Some(c);
            AccessPattern::Register(c)
        };

        let target = compiler.reg().at(self.span())?;
        let call = compiler.call(Call {
            callee: callee.clone(),
            args,
            target,
            math: in_math,
            trailing_comma,
        });

        compiler.spans.push(span);
        compiler.instructions.push(Instruction::Call { call });

        callee.free(compiler);
        compiler.free(args);
        if let Some(callee) = callee_reg {
            compiler.free(callee);
        }

        Ok(target)
    }
}

impl Compile for ast::Args<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        fn compile_arg(
            arg: ast::Arg<'_>,
            args: Register,
            compiler: &mut Compiler,
        ) -> SourceResult<()> {
            compiler.spans.push(arg.span());
            match arg {
                ast::Arg::Pos(pos) => {
                    let value = pos.compile(compiler)?;
                    compiler.instructions.push(Instruction::ArgsPush { args, value });
                    compiler.free(value);
                    Ok(())
                }
                ast::Arg::Named(named) => {
                    let key = compiler.string(named.name().get());
                    let value = named.expr().compile(compiler)?;
                    compiler.instructions.push(Instruction::ArgsInsert {
                        args,
                        key,
                        value,
                    });
                    compiler.free(value);
                    Ok(())
                }
                ast::Arg::Spread(spread) => {
                    let value = spread.compile(compiler)?;
                    compiler.instructions.push(Instruction::ArgsSpread { args, value });
                    compiler.free(value);
                    Ok(())
                }
            }
        }

        let mut items = self.items();
        let Some(first) = items.next() else {
            return Ok(Register::NONE);
        };

        // We first initialize the arguments register
        let args = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler.instructions.push(Instruction::Args { target: args });

        // Then we compile the first argument
        compile_arg(first, args, compiler)?;

        // Then we compile the rest of the arguments
        for arg in items {
            compile_arg(arg, args, compiler)?;
        }

        Ok(args)
    }
}

impl Compile for ast::Closure<'_> {
    fn compile(&self, compiler: &mut Compiler) -> SourceResult<Register> {
        // Evaluate default values of named parameters.
        let mut defaults = Vec::new();
        for param in self.params().children() {
            if let ast::Param::Named(named) = param {
                let reg = named.expr().compile(compiler)?;
                defaults.push(reg);
            }
        }

        let name = compiler.current_name.clone();
        let library = compiler.engine.world.library().clone().into_inner();
        // Create a new scope for the closure.
        let mut closure_compiler =
            Compiler::new(&mut compiler.engine, None, Some(library), true)
                .with_parent(&compiler.scopes);

        // Create the local such that the closure can use itself.
        let closure_local = if let Some(name) = compiler.current_name.clone() {
            Some(closure_compiler.local(self.span(), name))
        } else {
            None
        };

        // Build the parameter list of the closure.
        let mut params = EcoVec::new();
        let mut defaults_iter = defaults.iter();
        for param in self.params().children() {
            match param {
                ast::Param::Pos(pat) => {
                    // Compile the pattern.
                    let pattern = pat.compile(&mut closure_compiler, true)?;
                    let span = pattern.span;

                    // Load the argument.
                    let target = closure_compiler.reg().at(pat.span())?;
                    closure_compiler.spans.push(span);
                    closure_compiler.instructions.push(Instruction::LoadArg {
                        arg: ArgumentId(params.len() as u16),
                        target,
                    });

                    // Bind the argument to the pattern.
                    if let PatternKind::Single(PatternItem::Simple(
                        span,
                        AccessPattern::Local(ScopeId(0), id),
                        _,
                    )) = &pattern.kind
                    {
                        closure_compiler.spans.push(*span);
                        closure_compiler
                            .instructions
                            .push(Instruction::Store { scope: ScopeId::SELF, local: *id, value: target });
                    } else {
                        let pattern_id = closure_compiler.pattern(pattern);
                        closure_compiler.spans.push(span);
                        closure_compiler.instructions.push(Instruction::Destructure {
                            pattern: pattern_id,
                            value: target,
                        });
                    }

                    closure_compiler.free(target);

                    // Add the parameter to the list.
                    match pat {
                        ast::Pattern::Normal(ast::Expr::Ident(name)) => {
                            params.push(ClosureParam::Pos(name.get().clone()));
                        }
                        _ => params.push(ClosureParam::Pos(EcoString::from("anonymous"))),
                    }
                }
                ast::Param::Named(named) => {
                    // Create the local variable.
                    let name = named.name().get();
                    let local = closure_compiler.local(named.name().span(), name.clone());

                    // Load the argument.
                    let target = closure_compiler.reg().at(named.span())?;
                    closure_compiler.spans.push(named.name().span());
                    closure_compiler.instructions.push(Instruction::LoadArg {
                        arg: ArgumentId(params.len() as u16),
                        target,
                    });

                    // Bind the argument to the local variable.
                    closure_compiler.spans.push(named.name().span());
                    closure_compiler
                        .instructions
                        .push(Instruction::Store { scope: ScopeId::SELF, local, value: target });

                    // Add the parameter to the list.
                    params.push(ClosureParam::Named {
                        name: name.clone(),
                        default: defaults_iter.next().copied(),
                    });

                    closure_compiler.free(target);
                }
                ast::Param::Sink(sink) => {
                    let Some(name) = sink.name() else {
                        // Add the parameter to the list.
                        params.push(ClosureParam::Sink(sink.span(), EcoString::new()));
                        continue;
                    };

                    // Create the local variable.
                    let local = closure_compiler.local(name.span(), name.get().clone());

                    // Load the argument.
                    let target = closure_compiler.reg().at(sink.span())?;
                    closure_compiler.spans.push(sink.span());
                    closure_compiler.instructions.push(Instruction::LoadArg {
                        arg: ArgumentId(params.len() as u16),
                        target,
                    });

                    // Bind the argument to the local variable.
                    closure_compiler.spans.push(sink.span());
                    closure_compiler
                        .instructions
                        .push(Instruction::Store { scope: ScopeId::SELF, value: target, local });

                    // Add the parameter to the list.
                    params.push(ClosureParam::Sink(sink.span(), name.get().clone()));

                    closure_compiler.free(target);
                }
            }
        }

        // Compile the body of the closure.
        let output = self.body().compile(&mut closure_compiler)?;

        // Then we can create the compiled closure.
        let closure = CompiledClosure {
            this: closure_local,
            span: self.span(),
            name: name.unwrap_or_else(|| EcoString::from("anonymous closure")),
            output,
            params,
            captures: closure_compiler.captures_as_vec(),
            closures: closure_compiler.closures_as_vec(),
            constants: closure_compiler.consts_as_vec(),
            strings: closure_compiler.strings_as_vec(),
            patterns: closure_compiler.patterns_as_vec(),
            labels: closure_compiler.labels_as_vec(),
            locals: closure_compiler.scopes.0.borrow().scopes.top.len(),
            instructions: closure_compiler.instructions,
            spans: closure_compiler.spans,
            content_labels: closure_compiler.content_labels,
            calls: closure_compiler.calls,
            accesses: closure_compiler.accesses,
        };

        // Then we can add the closure to the compiler.
        let closure_id = compiler.closure(closure);

        // Then we had the closure to the instructions.
        let target = compiler.reg().at(self.span())?;
        compiler.spans.push(self.span());
        compiler
            .instructions
            .push(Instruction::InstantiateClosure { closure: closure_id, target });

        // We do need to free the default params at the end because
        // they must live until `InstantiateClosure` is called.
        for default in defaults {
            compiler.free(default);
        }

        Ok(target)
    }
}

#[derive(Debug, Clone, Hash, PartialEq)]
pub struct CompiledClosure {
    /// A local variable that holds the closure itself.
    pub this: Option<LocalId>,
    /// The span of the closure.
    pub span: Span,
    /// The name of the closure.
    pub name: EcoString,
    /// The output if there is no return statement.
    pub output: Register,
    /// The parameters of the closure.
    pub params: EcoVec<ClosureParam>,
    /// The instructions that make up the closure.
    pub instructions: EcoVec<Instruction>,
    /// The spans of the instructions.
    pub spans: EcoVec<Span>,
    /// The calls of the closure.
    pub calls: EcoVec<Call>,
    /// The captured variables.
    pub captures: EcoVec<Capture>,
    /// The number of local variables.
    pub locals: usize,
    /// The constants of the closure.
    pub constants: EcoVec<Value>,
    /// The strings of the closure.
    pub strings: EcoVec<EcoString>,
    /// The patterns of the closure.
    pub patterns: EcoVec<Pattern>,
    /// The closures of the closure.
    pub closures: EcoVec<CompiledClosure>,
    /// The labels of the closure.
    pub labels: EcoVec<usize>,
    /// The content labels of the closure.
    pub content_labels: EcoVec<Label>,
    /// The accesses of the closure.
    pub accesses: EcoVec<AccessPattern>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ClosureParam {
    /// A positional parameter.
    Pos(EcoString),
    /// A named parameter.
    Named {
        /// The name of the parameter.
        name: EcoString,
        /// The default value of the parameter.
        default: Option<Register>,
    },
    /// A sink parameter.
    Sink(Span, EcoString),
}

///
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Capture {
    Local {
        /// The scope of the local variable.
        scope: ScopeId,
        /// The local variable.
        local: LocalId,
    },
    Captured {
        /// Capture a captured value.
        captured: CapturedId,
    },
}

fn in_math(expr: ast::Expr) -> bool {
    match expr {
        ast::Expr::MathIdent(_) => true,
        ast::Expr::FieldAccess(access) => in_math(access.target()),
        _ => false,
    }
}

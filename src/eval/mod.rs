//! Evaluation of syntax trees.

#[macro_use]
mod array;
#[macro_use]
mod dict;
#[macro_use]
mod value;
mod capture;
mod function;
mod ops;
mod scope;
mod template;

pub use array::*;
pub use capture::*;
pub use dict::*;
pub use function::*;
pub use scope::*;
pub use template::*;
pub use value::*;

use std::collections::HashMap;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::diag::{Error, StrResult, Tracepoint, TypResult};
use crate::geom::{Angle, Fractional, Length, Relative};
use crate::image::ImageStore;
use crate::loading::Loader;
use crate::parse::parse;
use crate::source::{SourceId, SourceStore};
use crate::syntax::visit::Visit;
use crate::syntax::*;
use crate::util::EcoString;
use crate::Context;

/// Evaluate a parsed source file into a module.
pub fn eval(
    ctx: &mut Context,
    source: SourceId,
    ast: Rc<SyntaxTree>,
) -> TypResult<Module> {
    let mut ctx = EvalContext::new(ctx, source);
    let template = ast.eval(&mut ctx)?;
    Ok(Module { scope: ctx.scopes.top, template })
}

/// Caches evaluated modules.
pub type ModuleCache = HashMap<SourceId, Module>;

/// An evaluated module, ready for importing or execution.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The top-level definitions that were bound in this module.
    pub scope: Scope,
    /// The template defined by this module.
    pub template: Template,
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output>;
}

/// The context for evaluation.
pub struct EvalContext<'a> {
    /// The loader from which resources (files and images) are loaded.
    pub loader: &'a dyn Loader,
    /// Stores loaded source files.
    pub sources: &'a mut SourceStore,
    /// Stores decoded images.
    pub images: &'a mut ImageStore,
    /// Caches evaluated modules.
    pub modules: &'a mut ModuleCache,
    /// The active scopes.
    pub scopes: Scopes<'a>,
    /// The id of the currently evaluated source file.
    pub source: SourceId,
    /// The stack of imported files that led to evaluation of the current file.
    pub route: Vec<SourceId>,
    /// The expression map for the currently built template.
    pub map: ExprMap,
}

impl<'a> EvalContext<'a> {
    /// Create a new evaluation context.
    pub fn new(ctx: &'a mut Context, source: SourceId) -> Self {
        Self {
            loader: ctx.loader.as_ref(),
            sources: &mut ctx.sources,
            images: &mut ctx.images,
            modules: &mut ctx.modules,
            scopes: Scopes::new(Some(&ctx.std)),
            source,
            route: vec![],
            map: ExprMap::new(),
        }
    }

    /// Process an import of a module relative to the current location.
    pub fn import(&mut self, path: &str, span: Span) -> TypResult<SourceId> {
        // Load the source file.
        let full = self.relpath(path);
        let id = self.sources.load(&full).map_err(|err| {
            Error::boxed(self.source, span, match err.kind() {
                io::ErrorKind::NotFound => "file not found".into(),
                _ => format!("failed to load source file ({})", err),
            })
        })?;

        // Prevent cyclic importing.
        if self.source == id || self.route.contains(&id) {
            bail!(self.source, span, "cyclic import");
        }

        // Check whether the module was already loaded.
        if self.modules.get(&id).is_some() {
            return Ok(id);
        }

        // Parse the file.
        let source = self.sources.get(id);
        let ast = parse(&source)?;

        // Prepare the new context.
        let new_scopes = Scopes::new(self.scopes.base);
        let old_scopes = mem::replace(&mut self.scopes, new_scopes);
        self.route.push(self.source);
        self.source = id;

        // Evaluate the module.
        let result = Rc::new(ast).eval(self);

        // Restore the old context.
        let new_scopes = mem::replace(&mut self.scopes, old_scopes);
        self.source = self.route.pop().unwrap();

        // Add a tracepoint to the errors.
        let template = result.map_err(|mut errors| {
            for error in errors.iter_mut() {
                error.trace.push((self.source, span, Tracepoint::Import));
            }
            errors
        })?;

        // Save the evaluated module.
        let module = Module { scope: new_scopes.top, template };
        self.modules.insert(id, module);

        Ok(id)
    }

    /// Complete a path that is relative to the current file to be relative to
    /// the environment's current directory.
    pub fn relpath(&self, path: impl AsRef<Path>) -> PathBuf {
        self.sources
            .get(self.source)
            .path()
            .parent()
            .expect("is a file")
            .join(path)
    }
}

impl Eval for Rc<SyntaxTree> {
    type Output = Template;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        trait Walk {
            fn walk(&self, ctx: &mut EvalContext) -> TypResult<()>;
        }

        impl Walk for SyntaxTree {
            fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
                for node in self.iter() {
                    node.walk(ctx)?;
                }
                Ok(())
            }
        }

        impl Walk for SyntaxNode {
            fn walk(&self, ctx: &mut EvalContext) -> TypResult<()> {
                match self {
                    Self::Text(_) => {}
                    Self::Space => {}
                    Self::Linebreak(_) => {}
                    Self::Parbreak(_) => {}
                    Self::Strong(_) => {}
                    Self::Emph(_) => {}
                    Self::Raw(_) => {}
                    Self::Heading(n) => n.body.walk(ctx)?,
                    Self::List(n) => n.body.walk(ctx)?,
                    Self::Enum(n) => n.body.walk(ctx)?,
                    Self::Expr(n) => {
                        let value = n.eval(ctx)?;
                        ctx.map.insert(n as *const _, value);
                    }
                }
                Ok(())
            }
        }

        let map = {
            let prev = mem::take(&mut ctx.map);
            self.walk(ctx)?;
            mem::replace(&mut ctx.map, prev)
        };

        Ok(TemplateTree { tree: Rc::clone(self), map }.into())
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        Ok(match *self {
            Self::None(_) => Value::None,
            Self::Auto(_) => Value::Auto,
            Self::Bool(_, v) => Value::Bool(v),
            Self::Int(_, v) => Value::Int(v),
            Self::Float(_, v) => Value::Float(v),
            Self::Length(_, v, unit) => Value::Length(Length::with_unit(v, unit)),
            Self::Angle(_, v, unit) => Value::Angle(Angle::with_unit(v, unit)),
            Self::Percent(_, v) => Value::Relative(Relative::new(v / 100.0)),
            Self::Fractional(_, v) => Value::Fractional(Fractional::new(v)),
            Self::Str(_, ref v) => Value::Str(v.clone()),
            Self::Ident(ref v) => match ctx.scopes.get(&v) {
                Some(slot) => slot.borrow().clone(),
                None => bail!(ctx.source, v.span, "unknown variable"),
            },
            Self::Array(ref v) => Value::Array(v.eval(ctx)?),
            Self::Dict(ref v) => Value::Dict(v.eval(ctx)?),
            Self::Template(ref v) => Value::Template(v.eval(ctx)?),
            Self::Group(ref v) => v.eval(ctx)?,
            Self::Block(ref v) => v.eval(ctx)?,
            Self::Call(ref v) => v.eval(ctx)?,
            Self::Closure(ref v) => v.eval(ctx)?,
            Self::With(ref v) => v.eval(ctx)?,
            Self::Unary(ref v) => v.eval(ctx)?,
            Self::Binary(ref v) => v.eval(ctx)?,
            Self::Let(ref v) => v.eval(ctx)?,
            Self::If(ref v) => v.eval(ctx)?,
            Self::While(ref v) => v.eval(ctx)?,
            Self::For(ref v) => v.eval(ctx)?,
            Self::Import(ref v) => v.eval(ctx)?,
            Self::Include(ref v) => v.eval(ctx)?,
        })
    }
}

impl Eval for ArrayExpr {
    type Output = Array;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        self.items.iter().map(|expr| expr.eval(ctx)).collect()
    }
}

impl Eval for DictExpr {
    type Output = Dict;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        self.items
            .iter()
            .map(|Named { name, expr }| Ok((name.string.clone(), expr.eval(ctx)?)))
            .collect()
    }
}

impl Eval for TemplateExpr {
    type Output = Template;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        self.tree.eval(ctx)
    }
}

impl Eval for GroupExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        self.expr.eval(ctx)
    }
}

impl Eval for BlockExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        if self.scoping {
            ctx.scopes.enter();
        }

        let mut output = Value::None;
        for expr in &self.exprs {
            let value = expr.eval(ctx)?;
            output = ops::join(output, value)
                .map_err(Error::partial(ctx.source, expr.span()))?;
        }

        if self.scoping {
            ctx.scopes.exit();
        }

        Ok(output)
    }
}

impl Eval for UnaryExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let value = self.expr.eval(ctx)?;
        let result = match self.op {
            UnOp::Pos => ops::pos(value),
            UnOp::Neg => ops::neg(value),
            UnOp::Not => ops::not(value),
        };
        result.map_err(Error::partial(ctx.source, self.span))
    }
}

impl Eval for BinaryExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        match self.op {
            BinOp::Add => self.apply(ctx, ops::add),
            BinOp::Sub => self.apply(ctx, ops::sub),
            BinOp::Mul => self.apply(ctx, ops::mul),
            BinOp::Div => self.apply(ctx, ops::div),
            BinOp::And => self.apply(ctx, ops::and),
            BinOp::Or => self.apply(ctx, ops::or),
            BinOp::Eq => self.apply(ctx, ops::eq),
            BinOp::Neq => self.apply(ctx, ops::neq),
            BinOp::Lt => self.apply(ctx, ops::lt),
            BinOp::Leq => self.apply(ctx, ops::leq),
            BinOp::Gt => self.apply(ctx, ops::gt),
            BinOp::Geq => self.apply(ctx, ops::geq),
            BinOp::Assign => self.assign(ctx, |_, b| Ok(b)),
            BinOp::AddAssign => self.assign(ctx, ops::add),
            BinOp::SubAssign => self.assign(ctx, ops::sub),
            BinOp::MulAssign => self.assign(ctx, ops::mul),
            BinOp::DivAssign => self.assign(ctx, ops::div),
            BinOp::Range => self.apply(ctx, ops::range),
        }
    }
}

impl BinaryExpr {
    /// Apply a basic binary operation.
    fn apply<F>(&self, ctx: &mut EvalContext, op: F) -> TypResult<Value>
    where
        F: FnOnce(Value, Value) -> StrResult<Value>,
    {
        let lhs = self.lhs.eval(ctx)?;

        // Short-circuit boolean operations.
        if (self.op == BinOp::And && lhs == Value::Bool(false))
            || (self.op == BinOp::Or && lhs == Value::Bool(true))
        {
            return Ok(lhs);
        }

        let rhs = self.rhs.eval(ctx)?;
        op(lhs, rhs).map_err(Error::partial(ctx.source, self.span))
    }

    /// Apply an assignment operation.
    fn assign<F>(&self, ctx: &mut EvalContext, op: F) -> TypResult<Value>
    where
        F: FnOnce(Value, Value) -> StrResult<Value>,
    {
        let lspan = self.lhs.span();
        let slot = if let Expr::Ident(id) = self.lhs.as_ref() {
            match ctx.scopes.get(id) {
                Some(slot) => Rc::clone(slot),
                None => bail!(ctx.source, lspan, "unknown variable"),
            }
        } else {
            bail!(ctx.source, lspan, "cannot assign to this expression",);
        };

        let rhs = self.rhs.eval(ctx)?;
        let mut mutable = match slot.try_borrow_mut() {
            Ok(mutable) => mutable,
            Err(_) => {
                bail!(ctx.source, lspan, "cannot assign to a constant",);
            }
        };

        let lhs = mem::take(&mut *mutable);
        *mutable = op(lhs, rhs).map_err(Error::partial(ctx.source, self.span))?;

        Ok(Value::None)
    }
}

impl Eval for CallExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let callee = self
            .callee
            .eval(ctx)?
            .cast::<Function>()
            .map_err(Error::partial(ctx.source, self.callee.span()))?;

        let mut args = self.args.eval(ctx)?;
        let returned = callee(ctx, &mut args).map_err(|mut errors| {
            for error in errors.iter_mut() {
                // Skip errors directly related to arguments.
                if error.source == ctx.source && self.span.contains(error.span) {
                    continue;
                }

                error.trace.push((
                    ctx.source,
                    self.span,
                    Tracepoint::Call(callee.name().map(Into::into)),
                ));
            }
            errors
        })?;

        args.finish()?;

        Ok(returned)
    }
}

impl Eval for CallArgs {
    type Output = FuncArgs;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        Ok(FuncArgs {
            source: ctx.source,
            span: self.span,
            items: self
                .items
                .iter()
                .map(|arg| arg.eval(ctx))
                .collect::<TypResult<Vec<_>>>()?,
        })
    }
}

impl Eval for CallArg {
    type Output = FuncArg;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        Ok(match self {
            Self::Pos(expr) => FuncArg {
                span: self.span(),
                name: None,
                value: Spanned::new(expr.eval(ctx)?, expr.span()),
            },
            Self::Named(Named { name, expr }) => FuncArg {
                span: self.span(),
                name: Some(name.string.clone()),
                value: Spanned::new(expr.eval(ctx)?, expr.span()),
            },
        })
    }
}

impl Eval for ClosureExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let file = ctx.source;
        let params = Rc::clone(&self.params);
        let body = Rc::clone(&self.body);

        // Collect the captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&ctx.scopes);
            visitor.visit_closure(self);
            visitor.finish()
        };

        let name = self.name.as_ref().map(|name| name.string.clone());
        let func = Function::new(name, move |ctx, args| {
            // Don't leak the scopes from the call site. Instead, we use the
            // scope of captured variables we collected earlier.
            let prev_scopes = mem::take(&mut ctx.scopes);
            let prev_file = mem::replace(&mut ctx.source, file);
            ctx.scopes.top = captured.clone();

            for param in params.iter() {
                let value = args.expect::<Value>(param.as_str())?;
                ctx.scopes.def_mut(param.as_str(), value);
            }

            let result = body.eval(ctx);
            ctx.scopes = prev_scopes;
            ctx.source = prev_file;
            result
        });

        Ok(Value::Func(func))
    }
}

impl Eval for WithExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let callee = self
            .callee
            .eval(ctx)?
            .cast::<Function>()
            .map_err(Error::partial(ctx.source, self.callee.span()))?;

        let applied = self.args.eval(ctx)?;

        let name = callee.name().cloned();
        let func = Function::new(name, move |ctx, args| {
            // Remove named arguments that were overridden.
            let kept: Vec<_> = applied
                .items
                .iter()
                .filter(|arg| {
                    arg.name.is_none()
                        || args.items.iter().all(|other| arg.name != other.name)
                })
                .cloned()
                .collect();

            // Preprend the applied arguments so that the positional arguments
            // are in the right order.
            args.items.splice(.. 0, kept);

            // Call the original function.
            callee(ctx, args)
        });

        Ok(Value::Func(func))
    }
}

impl Eval for LetExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let value = match &self.init {
            Some(expr) => expr.eval(ctx)?,
            None => Value::None,
        };
        ctx.scopes.def_mut(self.binding.as_str(), value);
        Ok(Value::None)
    }
}

impl Eval for IfExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let condition = self
            .condition
            .eval(ctx)?
            .cast::<bool>()
            .map_err(Error::partial(ctx.source, self.condition.span()))?;

        if condition {
            self.if_body.eval(ctx)
        } else if let Some(else_body) = &self.else_body {
            else_body.eval(ctx)
        } else {
            Ok(Value::None)
        }
    }
}

impl Eval for WhileExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let mut output = Value::None;

        while self
            .condition
            .eval(ctx)?
            .cast::<bool>()
            .map_err(Error::partial(ctx.source, self.condition.span()))?
        {
            let value = self.body.eval(ctx)?;
            output = ops::join(output, value)
                .map_err(Error::partial(ctx.source, self.body.span()))?;
        }

        Ok(output)
    }
}

impl Eval for ForExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        macro_rules! iter {
            (for ($($binding:ident => $value:ident),*) in $iter:expr) => {{
                let mut output = Value::None;
                ctx.scopes.enter();

                #[allow(unused_parens)]
                for ($($value),*) in $iter {
                    $(ctx.scopes.def_mut($binding.as_str(), $value);)*

                    let value = self.body.eval(ctx)?;
                    output = ops::join(output, value)
                        .map_err(Error::partial(ctx.source, self.body.span()))?;
                }

                ctx.scopes.exit();
                Ok(output)
            }};
        }

        let iter = self.iter.eval(ctx)?;
        match (&self.pattern, iter) {
            (ForPattern::Value(v), Value::Str(string)) => {
                iter!(for (v => value) in string.chars().map(|c| Value::Str(c.into())))
            }
            (ForPattern::Value(v), Value::Array(array)) => {
                iter!(for (v => value) in array.into_iter())
            }
            (ForPattern::KeyValue(i, v), Value::Array(array)) => {
                iter!(for (i => idx, v => value) in array.into_iter().enumerate())
            }
            (ForPattern::Value(v), Value::Dict(dict)) => {
                iter!(for (v => value) in dict.into_iter().map(|p| p.1))
            }
            (ForPattern::KeyValue(k, v), Value::Dict(dict)) => {
                iter!(for (k => key, v => value) in dict.into_iter())
            }
            (ForPattern::KeyValue(_, _), Value::Str(_)) => {
                bail!(ctx.source, self.pattern.span(), "mismatched pattern");
            }
            (_, iter) => bail!(
                ctx.source,
                self.iter.span(),
                "cannot loop over {}",
                iter.type_name(),
            ),
        }
    }
}

impl Eval for ImportExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let path = self
            .path
            .eval(ctx)?
            .cast::<EcoString>()
            .map_err(Error::partial(ctx.source, self.path.span()))?;

        let file = ctx.import(&path, self.path.span())?;
        let module = &ctx.modules[&file];

        match &self.imports {
            Imports::Wildcard => {
                for (var, slot) in module.scope.iter() {
                    ctx.scopes.def_mut(var, slot.borrow().clone());
                }
            }
            Imports::Idents(idents) => {
                for ident in idents {
                    if let Some(slot) = module.scope.get(&ident) {
                        ctx.scopes.def_mut(ident.as_str(), slot.borrow().clone());
                    } else {
                        bail!(ctx.source, ident.span, "unresolved import");
                    }
                }
            }
        }

        Ok(Value::None)
    }
}

impl Eval for IncludeExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let path = self
            .path
            .eval(ctx)?
            .cast::<EcoString>()
            .map_err(Error::partial(ctx.source, self.path.span()))?;

        let file = ctx.import(&path, self.path.span())?;
        let module = &ctx.modules[&file];

        Ok(Value::Template(module.template.clone()))
    }
}

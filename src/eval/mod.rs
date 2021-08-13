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

use std::cell::RefMut;
use std::collections::HashMap;
use std::io;
use std::mem;
use std::path::PathBuf;
use std::rc::Rc;

use crate::diag::{At, Error, StrResult, Trace, Tracepoint, TypResult};
use crate::geom::{Angle, Fractional, Length, Relative};
use crate::image::ImageStore;
use crate::loading::Loader;
use crate::parse::parse;
use crate::source::{SourceId, SourceStore};
use crate::syntax::visit::Visit;
use crate::syntax::*;
use crate::util::{EcoString, RefMutExt};
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
    /// The stack of imported files that led to evaluation of the current file.
    pub route: Vec<SourceId>,
    /// The active scopes.
    pub scopes: Scopes<'a>,
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
            route: vec![source],
            scopes: Scopes::new(Some(&ctx.std)),
            map: ExprMap::new(),
        }
    }

    /// Process an import of a module relative to the current location.
    pub fn import(&mut self, path: &str, span: Span) -> TypResult<SourceId> {
        // Load the source file.
        let full = self.make_path(path);
        let id = self.sources.load(&full).map_err(|err| {
            Error::boxed(span, match err.kind() {
                io::ErrorKind::NotFound => "file not found".into(),
                _ => format!("failed to load source file ({})", err),
            })
        })?;

        // Prevent cyclic importing.
        if self.route.contains(&id) {
            bail!(span, "cyclic import");
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
        self.route.push(id);

        // Evaluate the module.
        let template = Rc::new(ast).eval(self).trace(|| Tracepoint::Import, span)?;

        // Restore the old context.
        let new_scopes = mem::replace(&mut self.scopes, old_scopes);
        self.route.pop().unwrap();

        // Save the evaluated module.
        let module = Module { scope: new_scopes.top, template };
        self.modules.insert(id, module);

        Ok(id)
    }

    /// Complete a user-entered path (relative to the source file) to be
    /// relative to the compilation environment's root.
    pub fn make_path(&self, path: &str) -> PathBuf {
        if let Some(&id) = self.route.last() {
            if let Some(dir) = self.sources.get(id).path().parent() {
                return dir.join(path);
            }
        }

        path.into()
    }
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output>;
}

impl Eval for Rc<SyntaxTree> {
    type Output = Template;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
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
            Self::Ident(ref v) => v.eval(ctx)?,
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

impl Eval for Ident {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        match ctx.scopes.get(self) {
            Some(slot) => Ok(slot.borrow().clone()),
            None => bail!(self.span, "unknown variable"),
        }
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
            output = ops::join(output, value).at(expr.span())?;
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
        result.at(self.span)
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
        op(lhs, rhs).at(self.span)
    }

    /// Apply an assignment operation.
    fn assign<F>(&self, ctx: &mut EvalContext, op: F) -> TypResult<Value>
    where
        F: FnOnce(Value, Value) -> StrResult<Value>,
    {
        let rhs = self.rhs.eval(ctx)?;
        let mut target = self.lhs.access(ctx)?;
        let lhs = mem::take(&mut *target);
        *target = op(lhs, rhs).at(self.span)?;
        Ok(Value::None)
    }
}

impl Eval for CallExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let callee = self.callee.eval(ctx)?;
        let mut args = self.args.eval(ctx)?;

        match callee {
            Value::Array(array) => {
                array.get(args.into_index()?).map(Value::clone).at(self.span)
            }

            Value::Dict(dict) => {
                dict.get(&args.into_key()?).map(Value::clone).at(self.span)
            }

            Value::Func(func) => {
                let point = || Tracepoint::Call(func.name().map(Into::into));
                let value = func(ctx, &mut args).trace(point, self.span)?;
                args.finish()?;
                Ok(value)
            }

            v => bail!(
                self.callee.span(),
                "expected function or collection, found {}",
                v.type_name(),
            ),
        }
    }
}

impl Eval for CallArgs {
    type Output = FuncArgs;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        Ok(FuncArgs {
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
        struct FuncParam {
            name: EcoString,
            default: Option<Value>,
        }

        // Evaluate default values for named parameters.
        let params: Vec<_> = self
            .params
            .iter()
            .map(|param| match param {
                ClosureParam::Pos(name) => {
                    Ok(FuncParam { name: name.string.clone(), default: None })
                }
                ClosureParam::Named(Named { name, expr }) => Ok(FuncParam {
                    name: name.string.clone(),
                    default: Some(expr.eval(ctx)?),
                }),
            })
            .collect::<TypResult<_>>()?;

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&ctx.scopes);
            visitor.visit_closure(self);
            visitor.finish()
        };

        // Clone the body expression so that we don't have a lifetime
        // dependence on the AST.
        let body = Rc::clone(&self.body);
        let name = self.name.as_ref().map(|name| name.string.clone());

        // Define the actual function.
        let func = Function::new(name, move |ctx, args| {
            // Don't leak the scopes from the call site. Instead, we use the
            // scope of captured variables we collected earlier.
            let prev_scopes = mem::take(&mut ctx.scopes);
            ctx.scopes.top = captured.clone();

            // Parse the arguments according to the parameter list.
            for param in &params {
                let value = match &param.default {
                    None => args.expect::<Value>(&param.name)?,
                    Some(default) => args
                        .named::<Value>(&param.name)?
                        .unwrap_or_else(|| default.clone()),
                };

                ctx.scopes.def_mut(&param.name, value);
            }

            let value = body.eval(ctx)?;

            ctx.scopes = prev_scopes;
            Ok(value)
        });

        Ok(Value::Func(func))
    }
}

impl Eval for WithExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let callee = self.callee.eval(ctx)?.cast::<Function>().at(self.callee.span())?;

        let name = callee.name().cloned();
        let applied = self.args.eval(ctx)?;

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
        let condition =
            self.condition.eval(ctx)?.cast::<bool>().at(self.condition.span())?;

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

        while self.condition.eval(ctx)?.cast::<bool>().at(self.condition.span())? {
            let value = self.body.eval(ctx)?;
            output = ops::join(output, value).at(self.body.span())?;
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
                        .at(self.body.span())?;
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
                bail!(self.pattern.span(), "mismatched pattern");
            }
            (_, iter) => {
                bail!(self.iter.span(), "cannot loop over {}", iter.type_name());
            }
        }
    }
}

impl Eval for ImportExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let path = self.path.eval(ctx)?.cast::<EcoString>().at(self.path.span())?;

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
                        bail!(ident.span, "unresolved import");
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
        let path = self.path.eval(ctx)?.cast::<EcoString>().at(self.path.span())?;

        let file = ctx.import(&path, self.path.span())?;
        let module = &ctx.modules[&file];

        Ok(Value::Template(module.template.clone()))
    }
}

/// Walk a node in a template, filling the context's expression map.
pub trait Walk {
    /// Walk the node.
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

/// Try to mutably access the value an expression points to.
///
/// This only works if the expression is a valid lvalue.
pub trait Access {
    /// Try to access the value.
    fn access<'a>(&self, ctx: &'a mut EvalContext) -> TypResult<RefMut<'a, Value>>;
}

impl Access for Expr {
    fn access<'a>(&self, ctx: &'a mut EvalContext) -> TypResult<RefMut<'a, Value>> {
        match self {
            Expr::Ident(ident) => ident.access(ctx),
            Expr::Call(call) => call.access(ctx),
            _ => bail!(self.span(), "cannot access this expression mutably"),
        }
    }
}

impl Access for Ident {
    fn access<'a>(&self, ctx: &'a mut EvalContext) -> TypResult<RefMut<'a, Value>> {
        match ctx.scopes.get(self) {
            Some(slot) => match slot.try_borrow_mut() {
                Ok(guard) => Ok(guard),
                Err(_) => bail!(self.span, "cannot mutate a constant"),
            },
            None => bail!(self.span, "unknown variable"),
        }
    }
}

impl Access for CallExpr {
    fn access<'a>(&self, ctx: &'a mut EvalContext) -> TypResult<RefMut<'a, Value>> {
        let args = self.args.eval(ctx)?;
        let guard = self.callee.access(ctx)?;

        RefMut::try_map(guard, |value| match value {
            Value::Array(array) => array.get_mut(args.into_index()?).at(self.span),

            Value::Dict(dict) => Ok(dict.get_mut(args.into_key()?)),

            v => bail!(
                self.callee.span(),
                "expected collection, found {}",
                v.type_name(),
            ),
        })
    }
}

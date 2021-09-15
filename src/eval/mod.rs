//! Evaluation of parsed markup into templates.

#[macro_use]
mod array;
#[macro_use]
mod dict;
#[macro_use]
mod str;
#[macro_use]
mod value;
mod capture;
mod function;
mod ops;
mod scope;
mod state;
mod template;
mod walk;

pub use self::str::*;
pub use array::*;
pub use capture::*;
pub use dict::*;
pub use function::*;
pub use scope::*;
pub use state::*;
pub use template::*;
pub use value::*;
pub use walk::*;

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
use crate::parse::{parse_code, parse_markup};
use crate::source::{SourceId, SourceStore};
use crate::syntax::visit::Visit;
use crate::syntax::*;
use crate::util::RefMutExt;
use crate::Context;

/// Evaluate a parsed markup file into a template.
pub fn eval(ctx: &mut Context, source: SourceId, markup: &Markup) -> TypResult<Template> {
    let mut ctx = EvalContext::new(ctx, source);
    markup.eval(&mut ctx)
}

/// The context for evaluation.
pub struct EvalContext<'a> {
    /// The loader from which resources (files and images) are loaded.
    pub loader: &'a dyn Loader,
    /// Stores loaded source files.
    pub sources: &'a mut SourceStore,
    /// Stores decoded images.
    pub images: &'a mut ImageStore,
    /// The stack of imported files that led to evaluation of the current file.
    pub route: Vec<SourceId>,
    /// Caches exports of evaluated code files.
    pub imports: HashMap<SourceId, Scope>,
    /// Caches includable markup files.
    pub includes: HashMap<SourceId, Template>,
    /// The active scopes.
    pub scopes: Scopes<'a>,
    /// The currently built template.
    pub template: Template,
}

impl<'a> EvalContext<'a> {
    /// Create a new evaluation context.
    pub fn new(ctx: &'a mut Context, source: SourceId) -> Self {
        Self {
            loader: ctx.loader.as_ref(),
            sources: &mut ctx.sources,
            images: &mut ctx.images,
            route: vec![source],
            imports: HashMap::new(),
            includes: HashMap::new(),
            scopes: Scopes::new(Some(&ctx.std)),
            template: Template::new(),
        }
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

impl Eval for Markup {
    type Output = Template;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let prev = mem::take(&mut ctx.template);

        ctx.template.save();
        self.walk(ctx)?;
        ctx.template.restore();

        Ok(mem::replace(&mut ctx.template, prev))
    }
}

impl Eval for Code {
    type Output = Scope;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let scopes = Scopes::new(ctx.scopes.base);
        let prev = mem::replace(&mut ctx.scopes, scopes);

        for expr in self {
            expr.eval(ctx)?;
        }

        let scopes = mem::replace(&mut ctx.scopes, prev);
        Ok(scopes.top)
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        match self {
            Self::Ident(v) => v.eval(ctx),
            Self::Lit(v) => v.eval(ctx),
            Self::Array(v) => v.eval(ctx).map(Value::Array),
            Self::Dict(v) => v.eval(ctx).map(Value::Dict),
            Self::Template(v) => v.eval(ctx).map(Value::Template),
            Self::Group(v) => v.eval(ctx),
            Self::Block(v) => v.eval(ctx),
            Self::Call(v) => v.eval(ctx),
            Self::Closure(v) => v.eval(ctx),
            Self::With(v) => v.eval(ctx),
            Self::Unary(v) => v.eval(ctx),
            Self::Binary(v) => v.eval(ctx),
            Self::Let(v) => v.eval(ctx),
            Self::If(v) => v.eval(ctx),
            Self::While(v) => v.eval(ctx),
            Self::For(v) => v.eval(ctx),
            Self::Import(v) => v.eval(ctx),
            Self::Include(v) => v.eval(ctx),
        }
    }
}

impl Eval for Lit {
    type Output = Value;

    fn eval(&self, _: &mut EvalContext) -> TypResult<Self::Output> {
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
            Self::Str(_, ref v) => Value::Str(v.into()),
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
            .map(|Named { name, expr }| Ok(((&name.string).into(), expr.eval(ctx)?)))
            .collect()
    }
}

impl Eval for TemplateExpr {
    type Output = Template;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        self.body.eval(ctx)
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
        ctx.scopes.enter();

        let mut output = Value::None;
        for expr in &self.code {
            let value = expr.eval(ctx)?;
            output = ops::join(output, value).at(expr.span())?;
        }

        ctx.scopes.exit();

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
                dict.get(args.into_key()?).map(Value::clone).at(self.span)
            }

            Value::Func(func) => {
                let point = || Tracepoint::Call(func.name().map(ToString::to_string));
                let value = func.call(ctx, &mut args).trace(point, self.span)?;
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
    type Output = Args;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let mut items = Vec::with_capacity(self.items.len());

        for arg in &self.items {
            let span = arg.span();
            match arg {
                CallArg::Pos(expr) => {
                    items.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(ctx)?, expr.span()),
                    });
                }
                CallArg::Named(Named { name, expr }) => {
                    items.push(Arg {
                        span,
                        name: Some((&name.string).into()),
                        value: Spanned::new(expr.eval(ctx)?, expr.span()),
                    });
                }
                CallArg::Spread(expr) => match expr.eval(ctx)? {
                    Value::Array(array) => {
                        items.extend(array.into_iter().map(|value| Arg {
                            span,
                            name: None,
                            value: Spanned::new(value, span),
                        }));
                    }
                    Value::Dict(dict) => {
                        items.extend(dict.into_iter().map(|(key, value)| Arg {
                            span,
                            name: Some(key),
                            value: Spanned::new(value, span),
                        }));
                    }
                    v => {
                        if let Value::Dyn(dynamic) = &v {
                            if let Some(args) = dynamic.downcast_ref::<Args>() {
                                items.extend(args.items.iter().cloned());
                                continue;
                            }
                        }

                        bail!(expr.span(), "cannot spread {}", v.type_name())
                    }
                },
            }
        }

        Ok(Args { span: self.span, items })
    }
}

impl Eval for ClosureExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> TypResult<Self::Output> {
        let name = self.name.as_ref().map(|name| name.string.clone());

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&ctx.scopes);
            visitor.visit_closure(self);
            visitor.finish()
        };

        let mut sink = None;
        let mut params = Vec::with_capacity(self.params.len());

        // Collect parameters and an optional sink parameter.
        for param in &self.params {
            match param {
                ClosureParam::Pos(name) => {
                    params.push((name.string.clone(), None));
                }
                ClosureParam::Named(Named { name, expr }) => {
                    params.push((name.string.clone(), Some(expr.eval(ctx)?)));
                }
                ClosureParam::Sink(name) => {
                    if sink.is_some() {
                        bail!(name.span, "only one argument sink is allowed");
                    }
                    sink = Some(name.string.clone());
                }
            }
        }

        // Clone the body expression so that we don't have a lifetime
        // dependence on the AST.
        let body = Rc::clone(&self.body);

        // Define the actual function.
        let func = Function::new(name, move |ctx, args| {
            // Don't leak the scopes from the call site. Instead, we use the
            // scope of captured variables we collected earlier.
            let prev_scopes = mem::take(&mut ctx.scopes);
            ctx.scopes.top = captured.clone();

            // Parse the arguments according to the parameter list.
            for (param, default) in &params {
                ctx.scopes.def_mut(param, match default {
                    None => args.expect::<Value>(param)?,
                    Some(default) => {
                        args.named::<Value>(param)?.unwrap_or_else(|| default.clone())
                    }
                });
            }

            // Put the remaining arguments into the sink.
            if let Some(sink) = &sink {
                ctx.scopes.def_mut(sink, args.take());
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
        let wrapped = self.callee.eval(ctx)?.cast::<Function>().at(self.callee.span())?;
        let applied = self.args.eval(ctx)?;

        let name = wrapped.name().cloned();
        let func = Function::new(name, move |ctx, args| {
            args.items.splice(.. 0, applied.items.iter().cloned());
            wrapped.call(ctx, args)
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
                iter!(for (v => value) in string.iter())
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
        // Load the file.
        let path = self.path.eval(ctx)?.cast::<Str>().at(self.path.span())?;
        let full = ctx.make_path(&path);
        let id = ctx.sources.load(&full).map_err(|err| {
            Error::boxed(self.path.span(), match err.kind() {
                io::ErrorKind::NotFound => "file not found".into(),
                _ => format!("failed to load code file ({})", err),
            })
        })?;

        // Prevent cyclic importing.
        if ctx.route.contains(&id) {
            bail!(self.span, "cyclic import");
        }

        // Parse and evaluate the code file if it's not already loaded.
        if !ctx.imports.contains_key(&id) {
            ctx.route.push(id);
            let source = ctx.sources.get(id);
            let code = parse_code(&source)?;
            let exports = code.eval(ctx).trace(|| Tracepoint::Import, self.span)?;
            ctx.imports.insert(id, exports);
            ctx.route.pop().unwrap();
        }

        let exports = &ctx.imports[&id];
        match &self.imports {
            Imports::Wildcard => {
                for (var, slot) in exports.iter() {
                    ctx.scopes.def_mut(var, slot.borrow().clone());
                }
            }
            Imports::Idents(idents) => {
                for ident in idents {
                    if let Some(slot) = exports.get(&ident) {
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
        // Load the file.
        let path = self.path.eval(ctx)?.cast::<Str>().at(self.path.span())?;
        let full = ctx.make_path(&path);
        let id = ctx.sources.load(&full).map_err(|err| {
            Error::boxed(self.path.span(), match err.kind() {
                io::ErrorKind::NotFound => "file not found".into(),
                _ => format!("failed to load markup file ({})", err),
            })
        })?;

        // Prevent cyclic including.
        if ctx.route.contains(&id) {
            bail!(self.span, "cyclic include");
        }

        // Parse and evaluate the markup file if it's not already loaded.
        if !ctx.includes.contains_key(&id) {
            ctx.route.push(id);
            let source = ctx.sources.get(id);
            let markup = parse_markup(&source)?;
            let scopes = Scopes::new(ctx.scopes.base);
            let prev = mem::replace(&mut ctx.scopes, scopes);
            let template = markup.eval(ctx).trace(|| Tracepoint::Include, self.span)?;
            ctx.scopes = prev;
            ctx.includes.insert(id, template);
            ctx.route.pop().unwrap();
        }

        Ok(Value::Template(ctx.includes[&id].clone()))
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

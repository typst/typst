//! Evaluation of syntax trees.

#[macro_use]
mod value;
mod capture;
mod ops;
mod scope;

pub use capture::*;
pub use scope::*;
pub use value::*;

use std::collections::HashMap;
use std::mem;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::cache::Cache;
use crate::diag::{Diag, DiagSet, Pass};
use crate::eco::EcoString;
use crate::geom::{Angle, Fractional, Length, Relative};
use crate::loading::{FileHash, Loader};
use crate::parse::parse;
use crate::syntax::visit::Visit;
use crate::syntax::*;
use crate::util::PathExt;

/// Evaluate a parsed source file into a module.
pub fn eval(
    loader: &mut dyn Loader,
    cache: &mut Cache,
    path: Option<&Path>,
    ast: Rc<SyntaxTree>,
    scope: &Scope,
) -> Pass<Module> {
    let mut ctx = EvalContext::new(loader, cache, path, scope);
    let template = ast.eval(&mut ctx);
    let module = Module { scope: ctx.scopes.top, template };
    Pass::new(module, ctx.diags)
}

/// An evaluated module, ready for importing or execution.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// The top-level definitions that were bound in this module.
    pub scope: Scope,
    /// The template defined by this module.
    pub template: TemplateValue,
}

/// The context for evaluation.
pub struct EvalContext<'a> {
    /// The loader from which resources (files and images) are loaded.
    pub loader: &'a mut dyn Loader,
    /// A cache for loaded resources.
    pub cache: &'a mut Cache,
    /// The active scopes.
    pub scopes: Scopes<'a>,
    /// Evaluation diagnostics.
    pub diags: DiagSet,
    /// The location of the currently evaluated file.
    pub path: Option<PathBuf>,
    /// The stack of imported files that led to evaluation of the current file.
    pub route: Vec<FileHash>,
    /// A map of loaded module.
    pub modules: HashMap<FileHash, Module>,
}

impl<'a> EvalContext<'a> {
    /// Create a new evaluation context with a base scope.
    pub fn new(
        loader: &'a mut dyn Loader,
        cache: &'a mut Cache,
        path: Option<&Path>,
        scope: &'a Scope,
    ) -> Self {
        let path = path.map(PathExt::normalize);

        let mut route = vec![];
        if let Some(path) = &path {
            if let Some(hash) = loader.resolve(path) {
                route.push(hash);
            }
        }

        Self {
            loader,
            cache,
            scopes: Scopes::new(Some(scope)),
            diags: DiagSet::new(),
            path,
            route,
            modules: HashMap::new(),
        }
    }

    /// Resolve a path relative to the current file.
    ///
    /// Generates an error if the file is not found.
    pub fn resolve(&mut self, path: &str, span: Span) -> Option<(PathBuf, FileHash)> {
        let path = match &self.path {
            Some(current) => current.parent()?.join(path),
            None => PathBuf::from(path),
        };

        match self.loader.resolve(&path) {
            Some(hash) => Some((path.normalize(), hash)),
            None => {
                self.diag(error!(span, "file not found"));
                None
            }
        }
    }

    /// Process an import of a module relative to the current location.
    pub fn import(&mut self, path: &str, span: Span) -> Option<FileHash> {
        let (resolved, hash) = self.resolve(path, span)?;

        // Prevent cyclic importing.
        if self.route.contains(&hash) {
            self.diag(error!(span, "cyclic import"));
            return None;
        }

        // Check whether the module was already loaded.
        if self.modules.get(&hash).is_some() {
            return Some(hash);
        }

        let buffer = self.loader.load_file(&resolved).or_else(|| {
            self.diag(error!(span, "failed to load file"));
            None
        })?;

        let string = std::str::from_utf8(&buffer).ok().or_else(|| {
            self.diag(error!(span, "file is not valid utf-8"));
            None
        })?;

        // Parse the file.
        let parsed = parse(string);

        // Prepare the new context.
        let new_scopes = Scopes::new(self.scopes.base);
        let old_scopes = mem::replace(&mut self.scopes, new_scopes);
        let old_diags = mem::replace(&mut self.diags, parsed.diags);
        let old_path = mem::replace(&mut self.path, Some(resolved));
        self.route.push(hash);

        // Evaluate the module.
        let ast = Rc::new(parsed.output);
        let template = ast.eval(self);

        // Restore the old context.
        let new_scopes = mem::replace(&mut self.scopes, old_scopes);
        let new_diags = mem::replace(&mut self.diags, old_diags);
        self.path = old_path;
        self.route.pop();

        // Put all diagnostics from the module on the import.
        for mut diag in new_diags {
            diag.span = span;
            self.diag(diag);
        }

        // Save the evaluated module.
        let module = Module { scope: new_scopes.top, template };
        self.modules.insert(hash, module);

        Some(hash)
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }

    /// Cast a value to a type and diagnose a possible error / warning.
    pub fn cast<T>(&mut self, value: Value, span: Span) -> Option<T>
    where
        T: Cast<Value>,
    {
        if value == Value::Error {
            return None;
        }

        match T::cast(value) {
            CastResult::Ok(t) => Some(t),
            CastResult::Warn(t, m) => {
                self.diag(warning!(span, "{}", m));
                Some(t)
            }
            CastResult::Err(value) => {
                self.diag(error!(
                    span,
                    "expected {}, found {}",
                    T::TYPE_NAME,
                    value.type_name(),
                ));
                None
            }
        }
    }
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, ctx: &mut EvalContext) -> Self::Output;
}

impl Eval for Rc<SyntaxTree> {
    type Output = TemplateValue;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        struct ExprVisitor<'a, 'b> {
            ctx: &'a mut EvalContext<'b>,
            map: ExprMap,
        }

        impl<'ast> Visit<'ast> for ExprVisitor<'_, '_> {
            fn visit_expr(&mut self, node: &'ast Expr) {
                self.map.insert(node as *const _, node.eval(self.ctx));
            }
        }

        let mut visitor = ExprVisitor { ctx, map: ExprMap::new() };
        visitor.visit_tree(self);

        Rc::new(vec![TemplateNode::Tree {
            tree: Rc::clone(self),
            map: visitor.map,
        }])
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match *self {
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
                None => {
                    ctx.diag(error!(v.span, "unknown variable"));
                    Value::Error
                }
            },
            Self::Array(ref v) => Value::Array(v.eval(ctx)),
            Self::Dict(ref v) => Value::Dict(v.eval(ctx)),
            Self::Template(ref v) => Value::Template(v.eval(ctx)),
            Self::Group(ref v) => v.eval(ctx),
            Self::Block(ref v) => v.eval(ctx),
            Self::Call(ref v) => v.eval(ctx),
            Self::Closure(ref v) => v.eval(ctx),
            Self::With(ref v) => v.eval(ctx),
            Self::Unary(ref v) => v.eval(ctx),
            Self::Binary(ref v) => v.eval(ctx),
            Self::Let(ref v) => v.eval(ctx),
            Self::If(ref v) => v.eval(ctx),
            Self::While(ref v) => v.eval(ctx),
            Self::For(ref v) => v.eval(ctx),
            Self::Import(ref v) => v.eval(ctx),
            Self::Include(ref v) => v.eval(ctx),
        }
    }
}

impl Eval for ArrayExpr {
    type Output = ArrayValue;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        self.items.iter().map(|expr| expr.eval(ctx)).collect()
    }
}

impl Eval for DictExpr {
    type Output = DictValue;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        self.items
            .iter()
            .map(|Named { name, expr }| (name.string.clone(), expr.eval(ctx)))
            .collect()
    }
}

impl Eval for TemplateExpr {
    type Output = TemplateValue;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        self.tree.eval(ctx)
    }
}

impl Eval for GroupExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        self.expr.eval(ctx)
    }
}

impl Eval for BlockExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        if self.scoping {
            ctx.scopes.enter();
        }

        let mut output = Value::None;
        for expr in &self.exprs {
            let value = expr.eval(ctx);
            output = output.join(ctx, value, expr.span());
        }

        if self.scoping {
            ctx.scopes.exit();
        }

        output
    }
}

impl Eval for UnaryExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let value = self.expr.eval(ctx);
        if value == Value::Error {
            return Value::Error;
        }

        let ty = value.type_name();
        let out = match self.op {
            UnOp::Pos => ops::pos(value),
            UnOp::Neg => ops::neg(value),
            UnOp::Not => ops::not(value),
        };

        if out == Value::Error {
            ctx.diag(error!(
                self.span,
                "cannot apply '{}' to {}",
                self.op.as_str(),
                ty,
            ));
        }

        out
    }
}

impl Eval for BinaryExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
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
            BinOp::Assign => self.assign(ctx, |_, b| b),
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
    fn apply<F>(&self, ctx: &mut EvalContext, op: F) -> Value
    where
        F: FnOnce(Value, Value) -> Value,
    {
        // Short-circuit boolean operations.
        let lhs = self.lhs.eval(ctx);
        match (self.op, &lhs) {
            (BinOp::And, Value::Bool(false)) => return lhs,
            (BinOp::Or, Value::Bool(true)) => return lhs,
            _ => {}
        }

        let rhs = self.rhs.eval(ctx);
        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        // Save type names before we consume the values in case of error.
        let types = (lhs.type_name(), rhs.type_name());
        let out = op(lhs, rhs);
        if out == Value::Error {
            self.error(ctx, types);
        }

        out
    }

    /// Apply an assignment operation.
    fn assign<F>(&self, ctx: &mut EvalContext, op: F) -> Value
    where
        F: FnOnce(Value, Value) -> Value,
    {
        let slot = if let Expr::Ident(id) = self.lhs.as_ref() {
            match ctx.scopes.get(id) {
                Some(slot) => Rc::clone(slot),
                None => {
                    ctx.diag(error!(self.lhs.span(), "unknown variable"));
                    return Value::Error;
                }
            }
        } else {
            ctx.diag(error!(self.lhs.span(), "cannot assign to this expression"));
            return Value::Error;
        };

        let rhs = self.rhs.eval(ctx);
        let mut mutable = match slot.try_borrow_mut() {
            Ok(mutable) => mutable,
            Err(_) => {
                ctx.diag(error!(self.lhs.span(), "cannot assign to a constant"));
                return Value::Error;
            }
        };

        let lhs = mem::take(&mut *mutable);
        let types = (lhs.type_name(), rhs.type_name());
        *mutable = op(lhs, rhs);

        if *mutable == Value::Error {
            self.error(ctx, types);
            return Value::Error;
        }

        Value::None
    }

    fn error(&self, ctx: &mut EvalContext, (a, b): (&str, &str)) {
        ctx.diag(error!(self.span, "{}", match self.op {
            BinOp::Add => format!("cannot add {} and {}", a, b),
            BinOp::Sub => format!("cannot subtract {1} from {0}", a, b),
            BinOp::Mul => format!("cannot multiply {} with {}", a, b),
            BinOp::Div => format!("cannot divide {} by {}", a, b),
            _ => format!("cannot apply '{}' to {} and {}", self.op.as_str(), a, b),
        }));
    }
}

impl Eval for CallExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let callee = self.callee.eval(ctx);
        if let Some(func) = ctx.cast::<FuncValue>(callee, self.callee.span()) {
            let mut args = self.args.eval(ctx);
            let returned = func(ctx, &mut args);
            args.finish(ctx);
            returned
        } else {
            Value::Error
        }
    }
}

impl Eval for CallArgs {
    type Output = FuncArgs;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let items = self.items.iter().map(|arg| arg.eval(ctx)).collect();
        FuncArgs { span: self.span, items }
    }
}

impl Eval for CallArg {
    type Output = FuncArg;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match self {
            Self::Pos(expr) => FuncArg {
                span: self.span(),
                name: None,
                value: Spanned::new(expr.eval(ctx), expr.span()),
            },
            Self::Named(Named { name, expr }) => FuncArg {
                span: self.span(),
                name: Some(name.string.clone()),
                value: Spanned::new(expr.eval(ctx), expr.span()),
            },
        }
    }
}

impl Eval for ClosureExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let params = Rc::clone(&self.params);
        let body = Rc::clone(&self.body);

        // Collect the captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&ctx.scopes);
            visitor.visit_closure(self);
            visitor.finish()
        };

        let name = self.name.as_ref().map(|name| name.string.clone());
        Value::Func(FuncValue::new(name, move |ctx, args| {
            // Don't leak the scopes from the call site. Instead, we use the
            // scope of captured variables we collected earlier.
            let prev = mem::take(&mut ctx.scopes);
            ctx.scopes.top = captured.clone();

            for param in params.iter() {
                // Set the parameter to `none` if the argument is missing.
                let value = args.expect::<Value>(ctx, param.as_str()).unwrap_or_default();
                ctx.scopes.def_mut(param.as_str(), value);
            }

            let value = body.eval(ctx);
            ctx.scopes = prev;
            value
        }))
    }
}

impl Eval for WithExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let callee = self.callee.eval(ctx);
        if let Some(func) = ctx.cast::<FuncValue>(callee, self.callee.span()) {
            let applied = self.args.eval(ctx);
            let name = func.name().cloned();
            Value::Func(FuncValue::new(name, move |ctx, args| {
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
                func(ctx, args)
            }))
        } else {
            Value::Error
        }
    }
}

impl Eval for LetExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let value = match &self.init {
            Some(expr) => expr.eval(ctx),
            None => Value::None,
        };
        ctx.scopes.def_mut(self.binding.as_str(), value);
        Value::None
    }
}

impl Eval for IfExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let condition = self.condition.eval(ctx);
        if let Some(condition) = ctx.cast(condition, self.condition.span()) {
            if condition {
                self.if_body.eval(ctx)
            } else if let Some(else_body) = &self.else_body {
                else_body.eval(ctx)
            } else {
                Value::None
            }
        } else {
            Value::Error
        }
    }
}

impl Eval for WhileExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let mut output = Value::None;
        loop {
            let condition = self.condition.eval(ctx);
            if let Some(condition) = ctx.cast(condition, self.condition.span()) {
                if condition {
                    let value = self.body.eval(ctx);
                    output = output.join(ctx, value, self.body.span());
                } else {
                    return output;
                }
            } else {
                return Value::Error;
            }
        }
    }
}

impl Eval for ForExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        macro_rules! iter {
            (for ($($binding:ident => $value:ident),*) in $iter:expr) => {{
                let mut output = Value::None;
                ctx.scopes.enter();

                #[allow(unused_parens)]
                for ($($value),*) in $iter {
                    $(ctx.scopes.def_mut($binding.as_str(), $value);)*

                    let value = self.body.eval(ctx);
                    output = output.join(ctx, value, self.body.span());
                }

                ctx.scopes.exit();
                output
            }};
        }

        let iter = self.iter.eval(ctx);
        match (self.pattern.clone(), iter) {
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
                ctx.diag(error!(self.pattern.span(), "mismatched pattern"));
                Value::Error
            }

            (_, iter) => {
                if iter != Value::Error {
                    ctx.diag(error!(
                        self.iter.span(),
                        "cannot loop over {}",
                        iter.type_name(),
                    ));
                }
                Value::Error
            }
        }
    }
}

impl Eval for ImportExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let path = self.path.eval(ctx);
        if let Some(path) = ctx.cast::<EcoString>(path, self.path.span()) {
            if let Some(hash) = ctx.import(&path, self.path.span()) {
                let mut module = &ctx.modules[&hash];
                match &self.imports {
                    Imports::Wildcard => {
                        for (var, slot) in module.scope.iter() {
                            let value = slot.borrow().clone();
                            ctx.scopes.def_mut(var, value);
                        }
                    }
                    Imports::Idents(idents) => {
                        for ident in idents {
                            if let Some(slot) = module.scope.get(&ident) {
                                let value = slot.borrow().clone();
                                ctx.scopes.def_mut(ident.as_str(), value);
                            } else {
                                ctx.diag(error!(ident.span, "unresolved import"));
                                module = &ctx.modules[&hash];
                            }
                        }
                    }
                }

                return Value::None;
            }
        }

        Value::Error
    }
}

impl Eval for IncludeExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let path = self.path.eval(ctx);
        if let Some(path) = ctx.cast::<EcoString>(path, self.path.span()) {
            if let Some(hash) = ctx.import(&path, self.path.span()) {
                return Value::Template(ctx.modules[&hash].template.clone());
            }
        }

        Value::Error
    }
}

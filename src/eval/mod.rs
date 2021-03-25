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
use std::rc::Rc;

use crate::color::Color;
use crate::diag::{Diag, DiagSet, Pass};
use crate::env::Env;
use crate::geom::{Angle, Length, Relative};
use crate::syntax::visit::Visit;
use crate::syntax::*;

/// Evaluate all nodes in a syntax tree.
///
/// The `scope` consists of the base definitions that are present from the
/// beginning (typically, the standard library).
pub fn eval(env: &mut Env, tree: &Tree, scope: &Scope) -> Pass<NodeMap> {
    let mut ctx = EvalContext::new(env, scope);
    let map = tree.eval(&mut ctx);
    Pass::new(map, ctx.diags)
}

/// A map from nodes to the values they evaluated to.
///
/// The raw pointers point into the nodes contained in some [`Tree`]. Since the
/// lifetime is erased, the tree could go out of scope while the hash map still
/// lives. Although this could lead to lookup panics, it is not unsafe since the
/// pointers are never dereferenced.
pub type NodeMap = HashMap<*const Node, Value>;

/// The context for evaluation.
pub struct EvalContext<'a> {
    /// The environment from which resources are gathered.
    pub env: &'a mut Env,
    /// The active scopes.
    pub scopes: Scopes<'a>,
    /// Evaluation diagnostics.
    pub diags: DiagSet,
}

impl<'a> EvalContext<'a> {
    /// Create a new execution context with a base scope.
    pub fn new(env: &'a mut Env, scope: &'a Scope) -> Self {
        Self {
            env,
            scopes: Scopes::with_base(scope),
            diags: DiagSet::new(),
        }
    }

    /// Add a diagnostic.
    pub fn diag(&mut self, diag: Diag) {
        self.diags.insert(diag);
    }
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, ctx: &mut EvalContext) -> Self::Output;
}

impl Eval for Tree {
    type Output = NodeMap;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let mut map = NodeMap::new();

        for node in self {
            let value = if let Some(call) = node.desugar() {
                call.eval(ctx)
            } else if let Node::Expr(expr) = node {
                expr.eval(ctx)
            } else {
                continue;
            };

            map.insert(node as *const _, value);
        }

        map
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match *self {
            Self::None(_) => Value::None,
            Self::Bool(_, v) => Value::Bool(v),
            Self::Int(_, v) => Value::Int(v),
            Self::Float(_, v) => Value::Float(v),
            Self::Length(_, v, unit) => Value::Length(Length::with_unit(v, unit)),
            Self::Angle(_, v, unit) => Value::Angle(Angle::with_unit(v, unit)),
            Self::Percent(_, v) => Value::Relative(Relative::new(v / 100.0)),
            Self::Color(_, v) => Value::Color(Color::Rgba(v)),
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
            Self::Template(ref v) => Value::Template(vec![v.eval(ctx)]),
            Self::Group(ref v) => v.eval(ctx),
            Self::Block(ref v) => v.eval(ctx),
            Self::Call(ref v) => v.eval(ctx),
            Self::Closure(ref v) => v.eval(ctx),
            Self::Unary(ref v) => v.eval(ctx),
            Self::Binary(ref v) => v.eval(ctx),
            Self::Let(ref v) => v.eval(ctx),
            Self::If(ref v) => v.eval(ctx),
            Self::While(ref v) => v.eval(ctx),
            Self::For(ref v) => v.eval(ctx),
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
    type Output = TemplateNode;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let tree = Rc::clone(&self.tree);
        let map = self.tree.eval(ctx);
        TemplateNode::Tree { tree, map }
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
            output = expr.eval(ctx);
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

        let lhs = std::mem::take(&mut *mutable);
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

        if let Value::Func(func) = callee {
            let func = func.clone();

            let mut args = self.args.eval(ctx);
            let returned = func(ctx, &mut args);
            args.finish(ctx);

            return returned;
        } else if callee != Value::Error {
            ctx.diag(error!(
                self.callee.span(),
                "expected function, found {}",
                callee.type_name(),
            ));
        }

        Value::Error
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
                name: None,
                value: Spanned::new(expr.eval(ctx), expr.span()),
            },
            Self::Named(Named { name, expr }) => FuncArg {
                name: Some(Spanned::new(name.string.clone(), name.span)),
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

        let name = self.name.as_ref().map(|id| id.to_string());
        Value::Func(FuncValue::new(name, move |ctx, args| {
            // Don't leak the scopes from the call site. Instead, we use the
            // scope of captured variables we collected earlier.
            let prev = std::mem::take(&mut ctx.scopes);
            ctx.scopes.top = captured.clone();

            for param in params.iter() {
                // Set the parameter to `none` if the argument is missing.
                let value =
                    args.require::<Value>(ctx, param.as_str()).unwrap_or_default();
                ctx.scopes.def_mut(param.as_str(), value);
            }

            let value = body.eval(ctx);
            ctx.scopes = prev;
            value
        }))
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
        if let Value::Bool(condition) = condition {
            if condition {
                self.if_body.eval(ctx)
            } else if let Some(else_body) = &self.else_body {
                else_body.eval(ctx)
            } else {
                Value::None
            }
        } else {
            if condition != Value::Error {
                ctx.diag(error!(
                    self.condition.span(),
                    "expected boolean, found {}",
                    condition.type_name(),
                ));
            }
            Value::Error
        }
    }
}

impl Eval for WhileExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let mut output = vec![];
        loop {
            let condition = self.condition.eval(ctx);
            if let Value::Bool(condition) = condition {
                if condition {
                    match self.body.eval(ctx) {
                        Value::Template(v) => output.extend(v),
                        Value::Str(v) => output.push(TemplateNode::Str(v)),
                        Value::Error => return Value::Error,
                        _ => {}
                    }
                } else {
                    return Value::Template(output);
                }
            } else {
                if condition != Value::Error {
                    ctx.diag(error!(
                        self.condition.span(),
                        "expected boolean, found {}",
                        condition.type_name(),
                    ));
                }
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
                let mut output = vec![];
                ctx.scopes.enter();

                #[allow(unused_parens)]
                for ($($value),*) in $iter {
                    $(ctx.scopes.def_mut($binding.as_str(), $value);)*

                    match self.body.eval(ctx) {
                        Value::Template(v) => output.extend(v),
                        Value::Str(v) => output.push(TemplateNode::Str(v)),
                        Value::Error => {
                            ctx.scopes.exit();
                            return Value::Error;
                        }
                        _ => {}
                    }
                }

                ctx.scopes.exit();
                Value::Template(output)
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

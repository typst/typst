//! Evaluation of markup into modules.

#[macro_use]
mod array;
#[macro_use]
mod dict;
#[macro_use]
mod value;

mod args;
mod capture;
mod func;
pub mod methods;
pub mod ops;
mod raw;
mod scope;
mod str;

pub use self::str::*;
pub use args::*;
pub use array::*;
pub use capture::*;
pub use dict::*;
pub use func::*;
pub use raw::*;
pub use scope::*;
pub use value::*;

use std::collections::BTreeMap;

use parking_lot::{MappedRwLockWriteGuard, RwLockWriteGuard};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{At, StrResult, Trace, Tracepoint, TypError, TypResult};
use crate::geom::{Angle, Em, Fraction, Length, Ratio};
use crate::library;
use crate::model::{Content, Pattern, Recipe, StyleEntry, StyleMap};
use crate::source::{SourceId, SourceStore};
use crate::syntax::ast::*;
use crate::syntax::{Span, Spanned};
use crate::util::EcoString;
use crate::Context;

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output>;
}

/// An evaluated module, ready for importing or layouting.
#[derive(Debug, Clone)]
pub struct Module {
    /// The top-level definitions that were bound in this module.
    pub scope: Scope,
    /// The module's layoutable contents.
    pub content: Content,
    /// The source file revisions this module depends on.
    pub deps: Vec<(SourceId, usize)>,
}

impl Module {
    /// Whether the module is still valid for the given sources.
    pub fn valid(&self, sources: &SourceStore) -> bool {
        self.deps.iter().all(|&(id, rev)| rev == sources.get(id).rev())
    }
}

/// A control flow event that occurred during evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Flow {
    /// Stop iteration in a loop.
    Break(Span),
    /// Skip the remainder of the current iteration in a loop.
    Continue(Span),
    /// Stop execution of a function early, optionally returning an explicit
    /// value.
    Return(Span, Option<Value>),
}

impl Flow {
    /// Return an error stating that this control flow is forbidden.
    pub fn forbidden(&self) -> TypError {
        match *self {
            Self::Break(span) => {
                error!(span, "cannot break outside of loop")
            }
            Self::Continue(span) => {
                error!(span, "cannot continue outside of loop")
            }
            Self::Return(span, _) => {
                error!(span, "cannot return outside of function")
            }
        }
    }
}

impl Eval for Markup {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        eval_markup(ctx, scp, &mut self.nodes())
    }
}

/// Evaluate a stream of markup nodes.
fn eval_markup(
    ctx: &mut Context,
    scp: &mut Scopes,
    nodes: &mut impl Iterator<Item = MarkupNode>,
) -> TypResult<Content> {
    let flow = ctx.flow.take();
    let mut seq = Vec::with_capacity(nodes.size_hint().1.unwrap_or_default());

    while let Some(node) = nodes.next() {
        seq.push(match node {
            MarkupNode::Expr(Expr::Set(set)) => {
                let styles = set.eval(ctx, scp)?;
                if ctx.flow.is_some() {
                    break;
                }

                eval_markup(ctx, scp, nodes)?.styled_with_map(styles)
            }
            MarkupNode::Expr(Expr::Show(show)) => {
                let recipe = show.eval(ctx, scp)?;
                if ctx.flow.is_some() {
                    break;
                }

                eval_markup(ctx, scp, nodes)?
                    .styled_with_entry(StyleEntry::Recipe(recipe).into())
            }
            MarkupNode::Expr(Expr::Wrap(wrap)) => {
                let tail = eval_markup(ctx, scp, nodes)?;
                scp.top.def_mut(wrap.binding().take(), tail);
                wrap.body().eval(ctx, scp)?.display()
            }

            _ => node.eval(ctx, scp)?,
        });

        if ctx.flow.is_some() {
            break;
        }
    }

    if flow.is_some() {
        ctx.flow = flow;
    }

    Ok(Content::sequence(seq))
}

impl Eval for MarkupNode {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        Ok(match self {
            Self::Space => Content::Space,
            Self::Parbreak => Content::Parbreak,
            &Self::Linebreak { justified } => Content::Linebreak { justified },
            Self::Text(text) => Content::Text(text.clone()),
            &Self::Quote { double } => Content::Quote { double },
            Self::Strong(strong) => strong.eval(ctx, scp)?,
            Self::Emph(emph) => emph.eval(ctx, scp)?,
            Self::Raw(raw) => raw.eval(ctx, scp)?,
            Self::Math(math) => math.eval(ctx, scp)?,
            Self::Heading(heading) => heading.eval(ctx, scp)?,
            Self::List(list) => list.eval(ctx, scp)?,
            Self::Enum(enum_) => enum_.eval(ctx, scp)?,
            Self::Expr(expr) => expr.eval(ctx, scp)?.display(),
        })
    }
}

impl Eval for StrongNode {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        Ok(Content::show(library::text::StrongNode(
            self.body().eval(ctx, scp)?,
        )))
    }
}

impl Eval for EmphNode {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        Ok(Content::show(library::text::EmphNode(
            self.body().eval(ctx, scp)?,
        )))
    }
}

impl Eval for RawNode {
    type Output = Content;

    fn eval(&self, _: &mut Context, _: &mut Scopes) -> TypResult<Self::Output> {
        let content = Content::show(library::text::RawNode {
            text: self.text.clone(),
            block: self.block,
        });
        Ok(match self.lang {
            Some(_) => content.styled(library::text::RawNode::LANG, self.lang.clone()),
            None => content,
        })
    }
}

impl Eval for MathNode {
    type Output = Content;

    fn eval(&self, _: &mut Context, _: &mut Scopes) -> TypResult<Self::Output> {
        Ok(Content::show(library::math::MathNode {
            formula: self.formula.clone(),
            display: self.display,
        }))
    }
}

impl Eval for HeadingNode {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        Ok(Content::show(library::structure::HeadingNode {
            body: self.body().eval(ctx, scp)?,
            level: self.level(),
        }))
    }
}

impl Eval for ListNode {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        Ok(Content::Item(library::structure::ListItem {
            kind: library::structure::UNORDERED,
            number: None,
            body: Box::new(self.body().eval(ctx, scp)?),
        }))
    }
}

impl Eval for EnumNode {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        Ok(Content::Item(library::structure::ListItem {
            kind: library::structure::ORDERED,
            number: self.number(),
            body: Box::new(self.body().eval(ctx, scp)?),
        }))
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let forbidden = |name| {
            error!(
                self.span(),
                "{} is only allowed directly in code and content blocks", name
            )
        };

        match self {
            Self::Lit(v) => v.eval(ctx, scp),
            Self::Ident(v) => v.eval(ctx, scp),
            Self::Code(v) => v.eval(ctx, scp),
            Self::Content(v) => v.eval(ctx, scp).map(Value::Content),
            Self::Array(v) => v.eval(ctx, scp).map(Value::Array),
            Self::Dict(v) => v.eval(ctx, scp).map(Value::Dict),
            Self::Group(v) => v.eval(ctx, scp),
            Self::FieldAccess(v) => v.eval(ctx, scp),
            Self::FuncCall(v) => v.eval(ctx, scp),
            Self::MethodCall(v) => v.eval(ctx, scp),
            Self::Closure(v) => v.eval(ctx, scp),
            Self::Unary(v) => v.eval(ctx, scp),
            Self::Binary(v) => v.eval(ctx, scp),
            Self::Let(v) => v.eval(ctx, scp),
            Self::Set(_) => Err(forbidden("set")),
            Self::Show(_) => Err(forbidden("show")),
            Self::Wrap(_) => Err(forbidden("wrap")),
            Self::If(v) => v.eval(ctx, scp),
            Self::While(v) => v.eval(ctx, scp),
            Self::For(v) => v.eval(ctx, scp),
            Self::Import(v) => v.eval(ctx, scp),
            Self::Include(v) => v.eval(ctx, scp).map(Value::Content),
            Self::Break(v) => v.eval(ctx, scp),
            Self::Continue(v) => v.eval(ctx, scp),
            Self::Return(v) => v.eval(ctx, scp),
        }
    }
}

impl Eval for Lit {
    type Output = Value;

    fn eval(&self, _: &mut Context, _: &mut Scopes) -> TypResult<Self::Output> {
        Ok(match self.kind() {
            LitKind::None => Value::None,
            LitKind::Auto => Value::Auto,
            LitKind::Bool(v) => Value::Bool(v),
            LitKind::Int(v) => Value::Int(v),
            LitKind::Float(v) => Value::Float(v),
            LitKind::Numeric(v, unit) => match unit {
                Unit::Length(unit) => Length::with_unit(v, unit).into(),
                Unit::Angle(unit) => Angle::with_unit(v, unit).into(),
                Unit::Em => Em::new(v).into(),
                Unit::Fr => Fraction::new(v).into(),
                Unit::Percent => Ratio::new(v / 100.0).into(),
            },
            LitKind::Str(ref v) => Value::Str(v.clone()),
        })
    }
}

impl Eval for Ident {
    type Output = Value;

    fn eval(&self, _: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        match scp.get(self) {
            Some(slot) => Ok(slot.read().clone()),
            None => bail!(self.span(), "unknown variable"),
        }
    }
}

impl Eval for CodeBlock {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        scp.enter();
        let output = eval_code(ctx, scp, &mut self.exprs())?;
        scp.exit();
        Ok(output)
    }
}

/// Evaluate a stream of expressions.
fn eval_code(
    ctx: &mut Context,
    scp: &mut Scopes,
    exprs: &mut impl Iterator<Item = Expr>,
) -> TypResult<Value> {
    let flow = ctx.flow.take();
    let mut output = Value::None;

    while let Some(expr) = exprs.next() {
        let span = expr.span();
        let value = match expr {
            Expr::Set(set) => {
                let styles = set.eval(ctx, scp)?;
                if ctx.flow.is_some() {
                    break;
                }

                let tail = eval_code(ctx, scp, exprs)?.display();
                Value::Content(tail.styled_with_map(styles))
            }
            Expr::Show(show) => {
                let recipe = show.eval(ctx, scp)?;
                let entry = StyleEntry::Recipe(recipe).into();
                if ctx.flow.is_some() {
                    break;
                }

                let tail = eval_code(ctx, scp, exprs)?.display();
                Value::Content(tail.styled_with_entry(entry))
            }
            Expr::Wrap(wrap) => {
                let tail = eval_code(ctx, scp, exprs)?;
                scp.top.def_mut(wrap.binding().take(), tail);
                wrap.body().eval(ctx, scp)?
            }

            _ => expr.eval(ctx, scp)?,
        };

        output = ops::join(output, value).at(span)?;

        if ctx.flow.is_some() {
            break;
        }
    }

    if flow.is_some() {
        ctx.flow = flow;
    }

    Ok(output)
}

impl Eval for ContentBlock {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        scp.enter();
        let content = self.body().eval(ctx, scp)?;
        scp.exit();
        Ok(content)
    }
}

impl Eval for GroupExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        self.expr().eval(ctx, scp)
    }
}

impl Eval for ArrayExpr {
    type Output = Array;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let items = self.items();

        let mut vec = Vec::with_capacity(items.size_hint().0);
        for item in items {
            match item {
                ArrayItem::Pos(expr) => vec.push(expr.eval(ctx, scp)?),
                ArrayItem::Spread(expr) => match expr.eval(ctx, scp)? {
                    Value::None => {}
                    Value::Array(array) => vec.extend(array.into_iter()),
                    v => bail!(expr.span(), "cannot spread {} into array", v.type_name()),
                },
            }
        }

        Ok(Array::from_vec(vec))
    }
}

impl Eval for DictExpr {
    type Output = Dict;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let mut map = BTreeMap::new();

        for item in self.items() {
            match item {
                DictItem::Named(named) => {
                    map.insert(named.name().take(), named.expr().eval(ctx, scp)?);
                }
                DictItem::Keyed(keyed) => {
                    map.insert(keyed.key(), keyed.expr().eval(ctx, scp)?);
                }
                DictItem::Spread(expr) => match expr.eval(ctx, scp)? {
                    Value::None => {}
                    Value::Dict(dict) => map.extend(dict.into_iter()),
                    v => bail!(
                        expr.span(),
                        "cannot spread {} into dictionary",
                        v.type_name()
                    ),
                },
            }
        }

        Ok(Dict::from_map(map))
    }
}

impl Eval for UnaryExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let value = self.expr().eval(ctx, scp)?;
        let result = match self.op() {
            UnOp::Pos => ops::pos(value),
            UnOp::Neg => ops::neg(value),
            UnOp::Not => ops::not(value),
        };
        Ok(result.at(self.span())?)
    }
}

impl Eval for BinaryExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        match self.op() {
            BinOp::Add => self.apply(ctx, scp, ops::add),
            BinOp::Sub => self.apply(ctx, scp, ops::sub),
            BinOp::Mul => self.apply(ctx, scp, ops::mul),
            BinOp::Div => self.apply(ctx, scp, ops::div),
            BinOp::And => self.apply(ctx, scp, ops::and),
            BinOp::Or => self.apply(ctx, scp, ops::or),
            BinOp::Eq => self.apply(ctx, scp, ops::eq),
            BinOp::Neq => self.apply(ctx, scp, ops::neq),
            BinOp::Lt => self.apply(ctx, scp, ops::lt),
            BinOp::Leq => self.apply(ctx, scp, ops::leq),
            BinOp::Gt => self.apply(ctx, scp, ops::gt),
            BinOp::Geq => self.apply(ctx, scp, ops::geq),
            BinOp::In => self.apply(ctx, scp, ops::in_),
            BinOp::NotIn => self.apply(ctx, scp, ops::not_in),
            BinOp::Assign => self.assign(ctx, scp, |_, b| Ok(b)),
            BinOp::AddAssign => self.assign(ctx, scp, ops::add),
            BinOp::SubAssign => self.assign(ctx, scp, ops::sub),
            BinOp::MulAssign => self.assign(ctx, scp, ops::mul),
            BinOp::DivAssign => self.assign(ctx, scp, ops::div),
        }
    }
}

impl BinaryExpr {
    /// Apply a basic binary operation.
    fn apply(
        &self,
        ctx: &mut Context,
        scp: &mut Scopes,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> TypResult<Value> {
        let lhs = self.lhs().eval(ctx, scp)?;

        // Short-circuit boolean operations.
        if (self.op() == BinOp::And && lhs == Value::Bool(false))
            || (self.op() == BinOp::Or && lhs == Value::Bool(true))
        {
            return Ok(lhs);
        }

        let rhs = self.rhs().eval(ctx, scp)?;
        Ok(op(lhs, rhs).at(self.span())?)
    }

    /// Apply an assignment operation.
    fn assign(
        &self,
        ctx: &mut Context,
        scp: &mut Scopes,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> TypResult<Value> {
        let rhs = self.rhs().eval(ctx, scp)?;
        let lhs = self.lhs();
        let mut location = lhs.access(ctx, scp)?;
        let lhs = std::mem::take(&mut *location);
        *location = op(lhs, rhs).at(self.span())?;
        Ok(Value::None)
    }
}

impl Eval for FieldAccess {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let object = self.object().eval(ctx, scp)?;
        let span = self.field().span();
        let field = self.field().take();

        Ok(match object {
            Value::Dict(dict) => dict.get(&field).at(span)?.clone(),

            Value::Content(Content::Show(_, Some(dict))) => dict
                .get(&field)
                .map_err(|_| format!("unknown field {field:?}"))
                .at(span)?
                .clone(),

            v => bail!(
                self.object().span(),
                "cannot access field on {}",
                v.type_name()
            ),
        })
    }
}

impl Eval for FuncCall {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let callee = self.callee().eval(ctx, scp)?;
        let args = self.args().eval(ctx, scp)?;

        Ok(match callee {
            Value::Array(array) => array.get(args.into_index()?).at(self.span())?.clone(),
            Value::Dict(dict) => dict.get(&args.into_key()?).at(self.span())?.clone(),
            Value::Func(func) => {
                let point = || Tracepoint::Call(func.name().map(ToString::to_string));
                func.call(ctx, args).trace(point, self.span())?
            }

            v => bail!(
                self.callee().span(),
                "expected callable or collection, found {}",
                v.type_name(),
            ),
        })
    }
}

impl Eval for MethodCall {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let span = self.span();
        let method = self.method();
        let point = || Tracepoint::Call(Some(method.to_string()));

        Ok(if methods::is_mutating(&method) {
            let args = self.args().eval(ctx, scp)?;
            let mut value = self.receiver().access(ctx, scp)?;
            methods::call_mut(ctx, &mut value, &method, args, span).trace(point, span)?;
            Value::None
        } else {
            let value = self.receiver().eval(ctx, scp)?;
            let args = self.args().eval(ctx, scp)?;
            methods::call(ctx, value, &method, args, span).trace(point, span)?
        })
    }
}

impl Eval for CallArgs {
    type Output = Args;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let mut items = Vec::new();

        for arg in self.items() {
            let span = arg.span();
            match arg {
                CallArg::Pos(expr) => {
                    items.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(ctx, scp)?, expr.span()),
                    });
                }
                CallArg::Named(named) => {
                    items.push(Arg {
                        span,
                        name: Some(named.name().take()),
                        value: Spanned::new(
                            named.expr().eval(ctx, scp)?,
                            named.expr().span(),
                        ),
                    });
                }
                CallArg::Spread(expr) => match expr.eval(ctx, scp)? {
                    Value::None => {}
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
                    Value::Args(args) => items.extend(args.items),
                    v => bail!(expr.span(), "cannot spread {}", v.type_name()),
                },
            }
        }

        Ok(Args { span: self.span(), items })
    }
}

impl Eval for ClosureExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        // The closure's name is defined by its let binding if there's one.
        let name = self.name().map(Ident::take);

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(scp);
            visitor.visit(self.as_red());
            visitor.finish()
        };

        let mut params = Vec::new();
        let mut sink = None;

        // Collect parameters and an optional sink parameter.
        for param in self.params() {
            match param {
                ClosureParam::Pos(name) => {
                    params.push((name.take(), None));
                }
                ClosureParam::Named(named) => {
                    params
                        .push((named.name().take(), Some(named.expr().eval(ctx, scp)?)));
                }
                ClosureParam::Sink(name) => {
                    if sink.is_some() {
                        bail!(name.span(), "only one argument sink is allowed");
                    }
                    sink = Some(name.take());
                }
            }
        }

        // Define the actual function.
        Ok(Value::Func(Func::from_closure(Closure {
            name,
            captured,
            params,
            sink,
            body: self.body(),
        })))
    }
}

impl Eval for LetExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let value = match self.init() {
            Some(expr) => expr.eval(ctx, scp)?,
            None => Value::None,
        };
        scp.top.def_mut(self.binding().take(), value);
        Ok(Value::None)
    }
}

impl Eval for SetExpr {
    type Output = StyleMap;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let target = self.target();
        let target = target.eval(ctx, scp)?.cast::<Func>().at(target.span())?;
        let args = self.args().eval(ctx, scp)?;
        Ok(target.set(args)?)
    }
}

impl Eval for ShowExpr {
    type Output = Recipe;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        // Evaluate the target function.
        let pattern = self.pattern();
        let pattern = pattern.eval(ctx, scp)?.cast::<Pattern>().at(pattern.span())?;

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(scp);
            visitor.visit(self.as_red());
            visitor.finish()
        };

        // Define parameters.
        let mut params = vec![];
        if let Some(binding) = self.binding() {
            params.push((binding.take(), None));
        }

        // Define the recipe function.
        let body = self.body();
        let span = body.span();
        let func = Func::from_closure(Closure {
            name: None,
            captured,
            params,
            sink: None,
            body,
        });

        Ok(Recipe { pattern, func, span })
    }
}

impl Eval for IfExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let condition = self.condition();
        if condition.eval(ctx, scp)?.cast::<bool>().at(condition.span())? {
            self.if_body().eval(ctx, scp)
        } else if let Some(else_body) = self.else_body() {
            else_body.eval(ctx, scp)
        } else {
            Ok(Value::None)
        }
    }
}

impl Eval for WhileExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let flow = ctx.flow.take();
        let mut output = Value::None;

        let condition = self.condition();
        while condition.eval(ctx, scp)?.cast::<bool>().at(condition.span())? {
            let body = self.body();
            let value = body.eval(ctx, scp)?;
            output = ops::join(output, value).at(body.span())?;

            match ctx.flow {
                Some(Flow::Break(_)) => {
                    ctx.flow = None;
                    break;
                }
                Some(Flow::Continue(_)) => ctx.flow = None,
                Some(Flow::Return(..)) => break,
                None => {}
            }
        }

        if flow.is_some() {
            ctx.flow = flow;
        }

        Ok(output)
    }
}

impl Eval for ForExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let flow = ctx.flow.take();
        let mut output = Value::None;
        scp.enter();

        macro_rules! iter {
            (for ($($binding:ident => $value:ident),*) in $iter:expr) => {{
                #[allow(unused_parens)]
                for ($($value),*) in $iter {
                    $(scp.top.def_mut(&$binding, $value);)*

                    let body = self.body();
                    let value = body.eval(ctx, scp)?;
                    output = ops::join(output, value).at(body.span())?;

                    match ctx.flow {
                        Some(Flow::Break(_)) => {
                            ctx.flow = None;
                            break;
                        }
                        Some(Flow::Continue(_)) => ctx.flow = None,
                        Some(Flow::Return(..)) => break,
                        None => {}
                    }
                }

            }};
        }

        let iter = self.iter().eval(ctx, scp)?;
        let pattern = self.pattern();
        let key = pattern.key().map(Ident::take);
        let value = pattern.value().take();

        match (key, value, iter) {
            (None, v, Value::Str(string)) => {
                iter!(for (v => value) in string.graphemes(true));
            }
            (None, v, Value::Array(array)) => {
                iter!(for (v => value) in array.into_iter());
            }
            (Some(i), v, Value::Array(array)) => {
                iter!(for (i => idx, v => value) in array.into_iter().enumerate());
            }
            (None, v, Value::Dict(dict)) => {
                iter!(for (v => value) in dict.into_iter().map(|p| p.1));
            }
            (Some(k), v, Value::Dict(dict)) => {
                iter!(for (k => key, v => value) in dict.into_iter());
            }
            (None, v, Value::Args(args)) => {
                iter!(for (v => value) in args.items.into_iter()
                    .filter(|arg| arg.name.is_none())
                    .map(|arg| arg.value.v));
            }
            (Some(k), v, Value::Args(args)) => {
                iter!(for (k => key, v => value) in args.items.into_iter()
                    .map(|arg| (arg.name.map_or(Value::None, Value::Str), arg.value.v)));
            }
            (_, _, Value::Str(_)) => {
                bail!(pattern.span(), "mismatched pattern");
            }
            (_, _, iter) => {
                bail!(self.iter().span(), "cannot loop over {}", iter.type_name());
            }
        }

        if flow.is_some() {
            ctx.flow = flow;
        }

        scp.exit();
        Ok(output)
    }
}

impl Eval for ImportExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let span = self.path().span();
        let path = self.path().eval(ctx, scp)?.cast::<EcoString>().at(span)?;
        let module = import(ctx, &path, span)?;

        match self.imports() {
            Imports::Wildcard => {
                for (var, slot) in module.scope.iter() {
                    scp.top.def_mut(var, slot.read().clone());
                }
            }
            Imports::Items(idents) => {
                for ident in idents {
                    if let Some(slot) = module.scope.get(&ident) {
                        scp.top.def_mut(ident.take(), slot.read().clone());
                    } else {
                        bail!(ident.span(), "unresolved import");
                    }
                }
            }
        }

        Ok(Value::None)
    }
}

impl Eval for IncludeExpr {
    type Output = Content;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let span = self.path().span();
        let path = self.path().eval(ctx, scp)?.cast::<EcoString>().at(span)?;
        let module = import(ctx, &path, span)?;
        Ok(module.content.clone())
    }
}

/// Process an import of a module relative to the current location.
fn import(ctx: &mut Context, path: &str, span: Span) -> TypResult<Module> {
    // Load the source file.
    let full = ctx.complete_path(path);
    let id = ctx.sources.load(&full).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound => error!(span, "file not found"),
        _ => error!(span, "failed to load source file ({})", err),
    })?;

    // Prevent cyclic importing.
    if ctx.route.contains(&id) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    let module = ctx.evaluate(id).trace(|| Tracepoint::Import, span)?;
    ctx.deps.extend(module.deps.iter().cloned());
    Ok(module)
}

impl Eval for BreakExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, _: &mut Scopes) -> TypResult<Self::Output> {
        if ctx.flow.is_none() {
            ctx.flow = Some(Flow::Break(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ContinueExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, _: &mut Scopes) -> TypResult<Self::Output> {
        if ctx.flow.is_none() {
            ctx.flow = Some(Flow::Continue(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ReturnExpr {
    type Output = Value;

    fn eval(&self, ctx: &mut Context, scp: &mut Scopes) -> TypResult<Self::Output> {
        let value = self.body().map(|body| body.eval(ctx, scp)).transpose()?;
        if ctx.flow.is_none() {
            ctx.flow = Some(Flow::Return(self.span(), value));
        }
        Ok(Value::None)
    }
}

/// Access an expression mutably.
pub trait Access {
    /// Access the value.
    fn access<'a>(
        &self,
        ctx: &mut Context,
        scp: &'a mut Scopes,
    ) -> TypResult<Location<'a>>;
}

impl Access for Expr {
    fn access<'a>(
        &self,
        ctx: &mut Context,
        scp: &'a mut Scopes,
    ) -> TypResult<Location<'a>> {
        match self {
            Expr::Ident(v) => v.access(ctx, scp),
            Expr::FieldAccess(v) => v.access(ctx, scp),
            Expr::FuncCall(v) => v.access(ctx, scp),
            _ => bail!(self.span(), "cannot mutate a temporary value"),
        }
    }
}

impl Access for Ident {
    fn access<'a>(
        &self,
        _: &mut Context,
        scp: &'a mut Scopes,
    ) -> TypResult<Location<'a>> {
        match scp.get(self) {
            Some(slot) => match slot.try_write() {
                Some(guard) => Ok(RwLockWriteGuard::map(guard, |v| v)),
                None => bail!(self.span(), "cannot mutate a constant"),
            },
            None => bail!(self.span(), "unknown variable"),
        }
    }
}

impl Access for FieldAccess {
    fn access<'a>(
        &self,
        ctx: &mut Context,
        scp: &'a mut Scopes,
    ) -> TypResult<Location<'a>> {
        let guard = self.object().access(ctx, scp)?;
        try_map(guard, |value| {
            Ok(match value {
                Value::Dict(dict) => dict.get_mut(self.field().take()),
                v => bail!(
                    self.object().span(),
                    "expected dictionary, found {}",
                    v.type_name(),
                ),
            })
        })
    }
}

impl Access for FuncCall {
    fn access<'a>(
        &self,
        ctx: &mut Context,
        scp: &'a mut Scopes,
    ) -> TypResult<Location<'a>> {
        let args = self.args().eval(ctx, scp)?;
        let guard = self.callee().access(ctx, scp)?;
        try_map(guard, |value| {
            Ok(match value {
                Value::Array(array) => {
                    array.get_mut(args.into_index()?).at(self.span())?
                }
                Value::Dict(dict) => dict.get_mut(args.into_key()?),
                v => bail!(
                    self.callee().span(),
                    "expected collection, found {}",
                    v.type_name(),
                ),
            })
        })
    }
}

/// A mutable location.
type Location<'a> = MappedRwLockWriteGuard<'a, Value>;

/// Map a reader-writer lock with a function.
fn try_map<F>(location: Location, f: F) -> TypResult<Location>
where
    F: FnOnce(&mut Value) -> TypResult<&mut Value>,
{
    let mut error = None;
    MappedRwLockWriteGuard::try_map(location, |value| match f(value) {
        Ok(value) => Some(value),
        Err(err) => {
            error = Some(err);
            None
        }
    })
    .map_err(|_| error.unwrap())
}

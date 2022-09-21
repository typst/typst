//! Evaluation of markup into modules.

#[macro_use]
mod cast;
#[macro_use]
mod array;
#[macro_use]
mod dict;
#[macro_use]
mod str;
#[macro_use]
mod value;
mod args;
mod capture;
mod func;
pub mod methods;
pub mod ops;
mod raw;
mod scope;
mod vm;

pub use self::str::*;
pub use args::*;
pub use array::*;
pub use capture::*;
pub use cast::*;
pub use dict::*;
pub use func::*;
pub use raw::*;
pub use scope::*;
pub use typst_macros::node;
pub use value::*;
pub use vm::*;

use std::collections::BTreeMap;

use comemo::{Track, Tracked};
use unicode_segmentation::UnicodeSegmentation;

use crate::diag::{At, SourceResult, StrResult, Trace, Tracepoint};
use crate::geom::{Angle, Em, Fraction, Length, Ratio};
use crate::library;
use crate::model::{Content, Pattern, Recipe, StyleEntry, StyleMap};
use crate::source::SourceId;
use crate::syntax::ast::*;
use crate::syntax::{Span, Spanned};
use crate::util::EcoString;
use crate::World;

/// Evaluate a source file and return the resulting module.
///
/// Returns either a module containing a scope with top-level bindings and
/// layoutable contents or diagnostics in the form of a vector of error
/// messages with file and span information.
#[comemo::memoize]
pub fn eval(
    world: Tracked<dyn World>,
    route: Tracked<Route>,
    id: SourceId,
) -> SourceResult<Module> {
    // Prevent cyclic evaluation.
    if route.contains(id) {
        let path = world.source(id).path().display();
        panic!("Tried to cyclicly evaluate {}", path);
    }

    // Evaluate the module.
    let route = unsafe { Route::insert(route, id) };
    let ast = world.source(id).ast()?;
    let std = &world.config().std;
    let scopes = Scopes::new(Some(std));
    let mut vm = Vm::new(world, route.track(), Some(id), scopes);
    let result = ast.eval(&mut vm);

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    // Assemble the module.
    Ok(Module { scope: vm.scopes.top, content: result? })
}

/// A route of source ids.
#[derive(Default)]
pub struct Route {
    parent: Option<Tracked<'static, Self>>,
    id: Option<SourceId>,
}

impl Route {
    /// Create a new, empty route.
    pub fn new(id: Option<SourceId>) -> Self {
        Self { id, parent: None }
    }

    /// Insert a new id into the route.
    ///
    /// You must guarantee that `outer` lives longer than the resulting
    /// route is ever used.
    unsafe fn insert(outer: Tracked<Route>, id: SourceId) -> Route {
        Route {
            parent: Some(std::mem::transmute(outer)),
            id: Some(id),
        }
    }
}

#[comemo::track]
impl Route {
    /// Whether the given id is part of the route.
    fn contains(&self, id: SourceId) -> bool {
        self.id == Some(id) || self.parent.map_or(false, |parent| parent.contains(id))
    }
}

/// An evaluated module, ready for importing or layouting.
#[derive(Debug, Clone)]
pub struct Module {
    /// The top-level definitions that were bound in this module.
    pub scope: Scope,
    /// The module's layoutable contents.
    pub content: Content,
}

/// Evaluate an expression.
pub trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output>;
}

impl Eval for Markup {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        eval_markup(vm, &mut self.nodes())
    }
}

/// Evaluate a stream of markup nodes.
fn eval_markup(
    vm: &mut Vm,
    nodes: &mut impl Iterator<Item = MarkupNode>,
) -> SourceResult<Content> {
    let flow = vm.flow.take();
    let mut seq = Vec::with_capacity(nodes.size_hint().1.unwrap_or_default());

    while let Some(node) = nodes.next() {
        seq.push(match node {
            MarkupNode::Expr(Expr::Set(set)) => {
                let styles = set.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                eval_markup(vm, nodes)?.styled_with_map(styles)
            }
            MarkupNode::Expr(Expr::Show(show)) => {
                let recipe = show.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                eval_markup(vm, nodes)?
                    .styled_with_entry(StyleEntry::Recipe(recipe).into())
            }
            MarkupNode::Expr(Expr::Wrap(wrap)) => {
                let tail = eval_markup(vm, nodes)?;
                vm.scopes.top.define(wrap.binding().take(), tail);
                wrap.body().eval(vm)?.display()
            }

            _ => node.eval(vm)?,
        });

        if vm.flow.is_some() {
            break;
        }
    }

    if flow.is_some() {
        vm.flow = flow;
    }

    Ok(Content::sequence(seq))
}

impl Eval for MarkupNode {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(match self {
            Self::Space => Content::Space,
            Self::Parbreak => Content::Parbreak,
            &Self::Linebreak { justified } => Content::Linebreak { justified },
            Self::Text(text) => Content::Text(text.clone()),
            &Self::Quote { double } => Content::Quote { double },
            Self::Strong(strong) => strong.eval(vm)?,
            Self::Emph(emph) => emph.eval(vm)?,
            Self::Raw(raw) => raw.eval(vm)?,
            Self::Math(math) => math.eval(vm)?,
            Self::Heading(heading) => heading.eval(vm)?,
            Self::List(list) => list.eval(vm)?,
            Self::Enum(enum_) => enum_.eval(vm)?,
            Self::Label(_) => Content::Empty,
            Self::Ref(label) => Content::show(library::structure::RefNode(label.clone())),
            Self::Expr(expr) => expr.eval(vm)?.display(),
        })
    }
}

impl Eval for StrongNode {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::show(library::text::StrongNode(
            self.body().eval(vm)?,
        )))
    }
}

impl Eval for EmphNode {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::show(library::text::EmphNode(
            self.body().eval(vm)?,
        )))
    }
}

impl Eval for RawNode {
    type Output = Content;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
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

impl Eval for Spanned<MathNode> {
    type Output = Content;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::show(library::math::MathNode {
            formula: self.clone().map(|math| math.formula),
            display: self.v.display,
        }))
    }
}

impl Eval for HeadingNode {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::show(library::structure::HeadingNode {
            body: self.body().eval(vm)?,
            level: self.level(),
        }))
    }
}

impl Eval for ListNode {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::Item(library::structure::ListItem {
            kind: library::structure::UNORDERED,
            number: None,
            body: Box::new(self.body().eval(vm)?),
        }))
    }
}

impl Eval for EnumNode {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::Item(library::structure::ListItem {
            kind: library::structure::ORDERED,
            number: self.number(),
            body: Box::new(self.body().eval(vm)?),
        }))
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let forbidden = |name| {
            error!(
                self.span(),
                "{} is only allowed directly in code and content blocks", name
            )
        };

        match self {
            Self::Lit(v) => v.eval(vm),
            Self::Ident(v) => v.eval(vm),
            Self::Code(v) => v.eval(vm),
            Self::Content(v) => v.eval(vm).map(Value::Content),
            Self::Array(v) => v.eval(vm).map(Value::Array),
            Self::Dict(v) => v.eval(vm).map(Value::Dict),
            Self::Group(v) => v.eval(vm),
            Self::FieldAccess(v) => v.eval(vm),
            Self::FuncCall(v) => v.eval(vm),
            Self::MethodCall(v) => v.eval(vm),
            Self::Closure(v) => v.eval(vm),
            Self::Unary(v) => v.eval(vm),
            Self::Binary(v) => v.eval(vm),
            Self::Let(v) => v.eval(vm),
            Self::Set(_) => bail!(forbidden("set")),
            Self::Show(_) => bail!(forbidden("show")),
            Self::Wrap(_) => bail!(forbidden("wrap")),
            Self::If(v) => v.eval(vm),
            Self::While(v) => v.eval(vm),
            Self::For(v) => v.eval(vm),
            Self::Import(v) => v.eval(vm),
            Self::Include(v) => v.eval(vm).map(Value::Content),
            Self::Break(v) => v.eval(vm),
            Self::Continue(v) => v.eval(vm),
            Self::Return(v) => v.eval(vm),
        }
    }
}

impl Eval for Lit {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
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
            LitKind::Str(v) => Value::Str(v.into()),
        })
    }
}

impl Eval for Ident {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.get(self).cloned().at(self.span())
    }
}

impl Eval for CodeBlock {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.enter();
        let output = eval_code(vm, &mut self.exprs())?;
        vm.scopes.exit();
        Ok(output)
    }
}

/// Evaluate a stream of expressions.
fn eval_code(vm: &mut Vm, exprs: &mut impl Iterator<Item = Expr>) -> SourceResult<Value> {
    let flow = vm.flow.take();
    let mut output = Value::None;

    while let Some(expr) = exprs.next() {
        let span = expr.span();
        let value = match expr {
            Expr::Set(set) => {
                let styles = set.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                let tail = eval_code(vm, exprs)?.display();
                Value::Content(tail.styled_with_map(styles))
            }
            Expr::Show(show) => {
                let recipe = show.eval(vm)?;
                let entry = StyleEntry::Recipe(recipe).into();
                if vm.flow.is_some() {
                    break;
                }

                let tail = eval_code(vm, exprs)?.display();
                Value::Content(tail.styled_with_entry(entry))
            }
            Expr::Wrap(wrap) => {
                let tail = eval_code(vm, exprs)?;
                vm.scopes.top.define(wrap.binding().take(), tail);
                wrap.body().eval(vm)?
            }

            _ => expr.eval(vm)?,
        };

        output = ops::join(output, value).at(span)?;

        if vm.flow.is_some() {
            break;
        }
    }

    if flow.is_some() {
        vm.flow = flow;
    }

    Ok(output)
}

impl Eval for ContentBlock {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.enter();
        let content = self.body().eval(vm)?;
        vm.scopes.exit();
        Ok(content)
    }
}

impl Eval for GroupExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        self.expr().eval(vm)
    }
}

impl Eval for ArrayExpr {
    type Output = Array;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let items = self.items();

        let mut vec = Vec::with_capacity(items.size_hint().0);
        for item in items {
            match item {
                ArrayItem::Pos(expr) => vec.push(expr.eval(vm)?),
                ArrayItem::Spread(expr) => match expr.eval(vm)? {
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let mut map = BTreeMap::new();

        for item in self.items() {
            match item {
                DictItem::Named(named) => {
                    map.insert(named.name().take().into(), named.expr().eval(vm)?);
                }
                DictItem::Keyed(keyed) => {
                    map.insert(keyed.key().into(), keyed.expr().eval(vm)?);
                }
                DictItem::Spread(expr) => match expr.eval(vm)? {
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.expr().eval(vm)?;
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        match self.op() {
            BinOp::Add => self.apply(vm, ops::add),
            BinOp::Sub => self.apply(vm, ops::sub),
            BinOp::Mul => self.apply(vm, ops::mul),
            BinOp::Div => self.apply(vm, ops::div),
            BinOp::And => self.apply(vm, ops::and),
            BinOp::Or => self.apply(vm, ops::or),
            BinOp::Eq => self.apply(vm, ops::eq),
            BinOp::Neq => self.apply(vm, ops::neq),
            BinOp::Lt => self.apply(vm, ops::lt),
            BinOp::Leq => self.apply(vm, ops::leq),
            BinOp::Gt => self.apply(vm, ops::gt),
            BinOp::Geq => self.apply(vm, ops::geq),
            BinOp::In => self.apply(vm, ops::in_),
            BinOp::NotIn => self.apply(vm, ops::not_in),
            BinOp::Assign => self.assign(vm, |_, b| Ok(b)),
            BinOp::AddAssign => self.assign(vm, ops::add),
            BinOp::SubAssign => self.assign(vm, ops::sub),
            BinOp::MulAssign => self.assign(vm, ops::mul),
            BinOp::DivAssign => self.assign(vm, ops::div),
        }
    }
}

impl BinaryExpr {
    /// Apply a basic binary operation.
    fn apply(
        &self,
        vm: &mut Vm,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> SourceResult<Value> {
        let lhs = self.lhs().eval(vm)?;

        // Short-circuit boolean operations.
        if (self.op() == BinOp::And && lhs == Value::Bool(false))
            || (self.op() == BinOp::Or && lhs == Value::Bool(true))
        {
            return Ok(lhs);
        }

        let rhs = self.rhs().eval(vm)?;
        Ok(op(lhs, rhs).at(self.span())?)
    }

    /// Apply an assignment operation.
    fn assign(
        &self,
        vm: &mut Vm,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> SourceResult<Value> {
        let rhs = self.rhs().eval(vm)?;
        let location = self.lhs().access(vm)?;
        let lhs = std::mem::take(&mut *location);
        *location = op(lhs, rhs).at(self.span())?;
        Ok(Value::None)
    }
}

impl Eval for FieldAccess {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let object = self.object().eval(vm)?;
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let callee = self.callee().eval(vm)?;
        let args = self.args().eval(vm)?;

        Ok(match callee {
            Value::Array(array) => array.get(args.into_index()?).at(self.span())?.clone(),
            Value::Dict(dict) => dict.get(&args.into_key()?).at(self.span())?.clone(),
            Value::Func(func) => {
                let point = || Tracepoint::Call(func.name().map(ToString::to_string));
                func.call(vm, args).trace(vm.world, point, self.span())?
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        let method = self.method();
        let point = || Tracepoint::Call(Some(method.to_string()));

        Ok(if methods::is_mutating(&method) {
            let args = self.args().eval(vm)?;
            let mut value = self.receiver().access(vm)?;
            methods::call_mut(&mut value, &method, args, span)
                .trace(vm.world, point, span)?;
            Value::None
        } else {
            let value = self.receiver().eval(vm)?;
            let args = self.args().eval(vm)?;
            methods::call(vm, value, &method, args, span).trace(vm.world, point, span)?
        })
    }
}

impl Eval for CallArgs {
    type Output = Args;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let mut items = Vec::new();

        for arg in self.items() {
            let span = arg.span();
            match arg {
                CallArg::Pos(expr) => {
                    items.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                CallArg::Named(named) => {
                    items.push(Arg {
                        span,
                        name: Some(named.name().take().into()),
                        value: Spanned::new(named.expr().eval(vm)?, named.expr().span()),
                    });
                }
                CallArg::Spread(expr) => match expr.eval(vm)? {
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        // The closure's name is defined by its let binding if there's one.
        let name = self.name().map(Ident::take);

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&vm.scopes);
            visitor.visit(self.as_untyped());
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
                    params.push((named.name().take(), Some(named.expr().eval(vm)?)));
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
            location: vm.location,
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = match self.init() {
            Some(expr) => expr.eval(vm)?,
            None => Value::None,
        };
        vm.scopes.top.define(self.binding().take(), value);
        Ok(Value::None)
    }
}

impl Eval for SetExpr {
    type Output = StyleMap;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let target = self.target();
        let target = target.eval(vm)?.cast::<Func>().at(target.span())?;
        let args = self.args().eval(vm)?;
        Ok(target.set(args)?)
    }
}

impl Eval for ShowExpr {
    type Output = Recipe;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        // Evaluate the target function.
        let pattern = self.pattern();
        let pattern = pattern.eval(vm)?.cast::<Pattern>().at(pattern.span())?;

        // Collect captured variables.
        let captured = {
            let mut visitor = CapturesVisitor::new(&vm.scopes);
            visitor.visit(self.as_untyped());
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
            location: vm.location,
            name: None,
            captured,
            params,
            sink: None,
            body,
        });

        Ok(Recipe { pattern, func: Spanned::new(func, span) })
    }
}

impl Eval for IfExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let condition = self.condition();
        if condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            self.if_body().eval(vm)
        } else if let Some(else_body) = self.else_body() {
            else_body.eval(vm)
        } else {
            Ok(Value::None)
        }
    }
}

impl Eval for WhileExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let flow = vm.flow.take();
        let mut output = Value::None;

        let condition = self.condition();
        while condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            let body = self.body();
            let value = body.eval(vm)?;
            output = ops::join(output, value).at(body.span())?;

            match vm.flow {
                Some(Flow::Break(_)) => {
                    vm.flow = None;
                    break;
                }
                Some(Flow::Continue(_)) => vm.flow = None,
                Some(Flow::Return(..)) => break,
                None => {}
            }
        }

        if flow.is_some() {
            vm.flow = flow;
        }

        Ok(output)
    }
}

impl Eval for ForExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let flow = vm.flow.take();
        let mut output = Value::None;
        vm.scopes.enter();

        macro_rules! iter {
            (for ($($binding:ident => $value:ident),*) in $iter:expr) => {{
                #[allow(unused_parens)]
                for ($($value),*) in $iter {
                    $(vm.scopes.top.define($binding.clone(), $value);)*

                    let body = self.body();
                    let value = body.eval(vm)?;
                    output = ops::join(output, value).at(body.span())?;

                    match vm.flow {
                        Some(Flow::Break(_)) => {
                            vm.flow = None;
                            break;
                        }
                        Some(Flow::Continue(_)) => vm.flow = None,
                        Some(Flow::Return(..)) => break,
                        None => {}
                    }
                }

            }};
        }

        let iter = self.iter().eval(vm)?;
        let pattern = self.pattern();
        let key = pattern.key().map(Ident::take);
        let value = pattern.value().take();

        match (key, value, iter) {
            (None, v, Value::Str(string)) => {
                iter!(for (v => value) in string.as_str().graphemes(true));
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
            vm.flow = flow;
        }

        vm.scopes.exit();
        Ok(output)
    }
}

impl Eval for ImportExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.path().span();
        let path = self.path().eval(vm)?.cast::<EcoString>().at(span)?;
        let module = import(vm, &path, span)?;

        match self.imports() {
            Imports::Wildcard => {
                for (var, value) in module.scope.iter() {
                    vm.scopes.top.define(var, value.clone());
                }
            }
            Imports::Items(idents) => {
                for ident in idents {
                    if let Some(value) = module.scope.get(&ident) {
                        vm.scopes.top.define(ident.take(), value.clone());
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

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.path().span();
        let path = self.path().eval(vm)?.cast::<EcoString>().at(span)?;
        let module = import(vm, &path, span)?;
        Ok(module.content.clone())
    }
}

/// Process an import of a module relative to the current location.
fn import(vm: &mut Vm, path: &str, span: Span) -> SourceResult<Module> {
    // Load the source file.
    let full = vm.locate(&path).at(span)?;
    let id = vm.world.resolve(&full).at(span)?;

    // Prevent cyclic importing.
    if vm.route.contains(id) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    let module =
        eval(vm.world, vm.route, id).trace(vm.world, || Tracepoint::Import, span)?;

    Ok(module)
}

impl Eval for BreakExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.flow.is_none() {
            vm.flow = Some(Flow::Break(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ContinueExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.flow.is_none() {
            vm.flow = Some(Flow::Continue(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ReturnExpr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.body().map(|body| body.eval(vm)).transpose()?;
        if vm.flow.is_none() {
            vm.flow = Some(Flow::Return(self.span(), value));
        }
        Ok(Value::None)
    }
}

/// Access an expression mutably.
pub trait Access {
    /// Access the value.
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value>;
}

impl Access for Expr {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        match self {
            Expr::Ident(v) => v.access(vm),
            Expr::FieldAccess(v) => v.access(vm),
            Expr::FuncCall(v) => v.access(vm),
            _ => bail!(self.span(), "cannot mutate a temporary value"),
        }
    }
}

impl Access for Ident {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        vm.scopes.get_mut(self).at(self.span())
    }
}

impl Access for FieldAccess {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        Ok(match self.object().access(vm)? {
            Value::Dict(dict) => dict.get_mut(self.field().take().into()),
            v => bail!(
                self.object().span(),
                "expected dictionary, found {}",
                v.type_name(),
            ),
        })
    }
}

impl Access for FuncCall {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        let args = self.args().eval(vm)?;
        Ok(match self.callee().access(vm)? {
            Value::Array(array) => array.get_mut(args.into_index()?).at(self.span())?,
            Value::Dict(dict) => dict.get_mut(args.into_key()?),
            v => bail!(
                self.callee().span(),
                "expected collection, found {}",
                v.type_name(),
            ),
        })
    }
}

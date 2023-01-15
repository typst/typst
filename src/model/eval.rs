//! Evaluation of markup into modules.

use std::collections::BTreeMap;
use std::mem;
use std::path::PathBuf;

use comemo::{Track, Tracked};
use unicode_segmentation::UnicodeSegmentation;

use super::{
    methods, ops, Arg, Args, Array, CapturesVisitor, Closure, Content, Dict, Func, Label,
    LangItems, Module, Recipe, Scopes, Selector, StyleMap, Transform, Value,
};
use crate::diag::{
    bail, error, At, SourceError, SourceResult, StrResult, Trace, Tracepoint,
};
use crate::geom::{Abs, Angle, Em, Fr, Ratio};
use crate::syntax::ast::AstNode;
use crate::syntax::{ast, Source, SourceId, Span, Spanned, SyntaxKind, SyntaxNode};
use crate::util::{EcoString, PathExt};
use crate::World;

const MAX_ITERATIONS: usize = 10_000;
const MAX_CALL_DEPTH: usize = 256;

/// Evaluate a source file and return the resulting module.
#[comemo::memoize]
pub fn eval(
    world: Tracked<dyn World>,
    route: Tracked<Route>,
    source: &Source,
) -> SourceResult<Module> {
    // Prevent cyclic evaluation.
    let id = source.id();
    if route.contains(id) {
        let path = world.source(id).path().display();
        panic!("Tried to cyclicly evaluate {}", path);
    }

    // Hook up the lang items.
    let library = world.library();
    super::set_lang_items(library.items.clone());

    // Evaluate the module.
    let route = unsafe { Route::insert(route, id) };
    let scopes = Scopes::new(Some(&library.scope));
    let mut vm = Vm::new(world, route.track(), id, scopes, 0);
    let result = source.ast()?.eval(&mut vm);

    // Handle control flow.
    if let Some(flow) = vm.flow {
        bail!(flow.forbidden());
    }

    // Assemble the module.
    Ok(Module::evaluated(source.path(), vm.scopes.top, result?))
}

/// A virtual machine.
///
/// Holds the state needed to [evaluate](eval) Typst sources. A new
/// virtual machine is created for each module evaluation and function call.
pub struct Vm<'a> {
    /// The compilation environment.
    pub(super) world: Tracked<'a, dyn World>,
    /// The language items.
    pub(super) items: LangItems,
    /// The route of source ids the VM took to reach its current location.
    pub(super) route: Tracked<'a, Route>,
    /// The current location.
    pub(super) location: SourceId,
    /// A control flow event that is currently happening.
    pub(super) flow: Option<Flow>,
    /// The stack of scopes.
    pub(super) scopes: Scopes<'a>,
    /// The current call depth.
    pub(super) depth: usize,
}

impl<'a> Vm<'a> {
    /// Create a new virtual machine.
    pub(super) fn new(
        world: Tracked<'a, dyn World>,
        route: Tracked<'a, Route>,
        location: SourceId,
        scopes: Scopes<'a>,
        depth: usize,
    ) -> Self {
        Self {
            world,
            items: world.library().items.clone(),
            route,
            location,
            flow: None,
            scopes,
            depth,
        }
    }

    /// Access the underlying world.
    pub fn world(&self) -> Tracked<'a, dyn World> {
        self.world
    }

    /// Resolve a user-entered path to be relative to the compilation
    /// environment's root.
    pub fn locate(&self, path: &str) -> StrResult<PathBuf> {
        if !self.location.is_detached() {
            if let Some(path) = path.strip_prefix('/') {
                return Ok(self.world.root().join(path).normalize());
            }

            if let Some(dir) = self.world.source(self.location).path().parent() {
                return Ok(dir.join(path).normalize());
            }
        }

        Err("cannot access file system from here".into())
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
    pub fn forbidden(&self) -> SourceError {
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

/// A route of source ids.
#[derive(Default)]
pub struct Route {
    parent: Option<Tracked<'static, Self>>,
    id: Option<SourceId>,
}

impl Route {
    /// Create a new route with just one entry.
    pub fn new(id: SourceId) -> Self {
        Self { id: Some(id), parent: None }
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

/// Evaluate an expression.
pub(super) trait Eval {
    /// The output of evaluating the expression.
    type Output;

    /// Evaluate the expression to the output value.
    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output>;
}

impl Eval for ast::Markup {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        eval_markup(vm, &mut self.exprs())
    }
}

/// Evaluate a stream of markup.
fn eval_markup(
    vm: &mut Vm,
    exprs: &mut impl Iterator<Item = ast::Expr>,
) -> SourceResult<Content> {
    let flow = vm.flow.take();
    let mut seq = Vec::with_capacity(exprs.size_hint().1.unwrap_or_default());

    while let Some(expr) = exprs.next() {
        match expr {
            ast::Expr::Set(set) => {
                let styles = set.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                seq.push(eval_markup(vm, exprs)?.styled_with_map(styles))
            }
            ast::Expr::Show(show) => {
                let recipe = show.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                let tail = eval_markup(vm, exprs)?;
                seq.push(tail.styled_with_recipe(vm.world, recipe)?)
            }
            expr => match expr.eval(vm)? {
                Value::Label(label) => {
                    if let Some(node) =
                        seq.iter_mut().rev().find(|node| node.labellable())
                    {
                        *node = mem::take(node).labelled(label);
                    }
                }
                value => seq.push(value.display().spanned(expr.span())),
            },
        }

        if vm.flow.is_some() {
            break;
        }
    }

    if flow.is_some() {
        vm.flow = flow;
    }

    Ok(Content::sequence(seq))
}

impl Eval for ast::Expr {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let forbidden = |name| {
            error!(
                self.span(),
                "{} is only allowed directly in code and content blocks", name
            )
        };

        match self {
            Self::Space(v) => v.eval(vm).map(Value::Content),
            Self::Linebreak(v) => v.eval(vm).map(Value::Content),
            Self::Text(v) => v.eval(vm).map(Value::Content),
            Self::Escape(v) => v.eval(vm).map(Value::Content),
            Self::Shorthand(v) => v.eval(vm).map(Value::Content),
            Self::Symbol(v) => v.eval(vm).map(Value::Content),
            Self::SmartQuote(v) => v.eval(vm).map(Value::Content),
            Self::Strong(v) => v.eval(vm).map(Value::Content),
            Self::Emph(v) => v.eval(vm).map(Value::Content),
            Self::Raw(v) => v.eval(vm).map(Value::Content),
            Self::Link(v) => v.eval(vm).map(Value::Content),
            Self::Label(v) => v.eval(vm),
            Self::Ref(v) => v.eval(vm).map(Value::Content),
            Self::Heading(v) => v.eval(vm).map(Value::Content),
            Self::List(v) => v.eval(vm).map(Value::Content),
            Self::Enum(v) => v.eval(vm).map(Value::Content),
            Self::Term(v) => v.eval(vm).map(Value::Content),
            Self::Atom(v) => v.eval(vm).map(Value::Content),
            Self::Script(v) => v.eval(vm).map(Value::Content),
            Self::Frac(v) => v.eval(vm).map(Value::Content),
            Self::AlignPoint(v) => v.eval(vm).map(Value::Content),
            Self::Ident(v) => v.eval(vm),
            Self::None(v) => v.eval(vm),
            Self::Auto(v) => v.eval(vm),
            Self::Bool(v) => v.eval(vm),
            Self::Int(v) => v.eval(vm),
            Self::Float(v) => v.eval(vm),
            Self::Numeric(v) => v.eval(vm),
            Self::Str(v) => v.eval(vm),
            Self::Code(v) => v.eval(vm),
            Self::Content(v) => v.eval(vm).map(Value::Content),
            Self::Math(v) => v.eval(vm).map(Value::Content),
            Self::Array(v) => v.eval(vm).map(Value::Array),
            Self::Dict(v) => v.eval(vm).map(Value::Dict),
            Self::Parenthesized(v) => v.eval(vm),
            Self::FieldAccess(v) => v.eval(vm),
            Self::FuncCall(v) => v.eval(vm),
            Self::MethodCall(v) => v.eval(vm),
            Self::Closure(v) => v.eval(vm),
            Self::Unary(v) => v.eval(vm),
            Self::Binary(v) => v.eval(vm),
            Self::Let(v) => v.eval(vm),
            Self::Set(_) => bail!(forbidden("set")),
            Self::Show(_) => bail!(forbidden("show")),
            Self::Conditional(v) => v.eval(vm),
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

impl ast::Expr {
    fn eval_in_math(&self, vm: &mut Vm) -> SourceResult<Content> {
        Ok(match self {
            Self::Escape(v) => v.eval_in_math(vm)?,
            Self::Shorthand(v) => v.eval_in_math(vm)?,
            Self::Symbol(v) => v.eval_in_math(vm)?,
            Self::Ident(v) => v.eval_in_math(vm)?,
            _ => self.eval(vm)?.display_in_math(),
        }
        .spanned(self.span()))
    }
}

impl Eval for ast::Space {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(match self.newlines() {
            0..=1 => (vm.items.space)(),
            _ => (vm.items.parbreak)(),
        })
    }
}

impl Eval for ast::Linebreak {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.linebreak)())
    }
}

impl Eval for ast::Text {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.text)(self.get().clone()))
    }
}

impl Eval for ast::Escape {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.text)(self.get().into()))
    }
}

impl ast::Escape {
    fn eval_in_math(&self, vm: &mut Vm) -> SourceResult<Content> {
        Ok((vm.items.math_atom)(self.get().into()))
    }
}

impl Eval for ast::Shorthand {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.text)(self.get().into()))
    }
}

impl ast::Shorthand {
    fn eval_in_math(&self, vm: &mut Vm) -> SourceResult<Content> {
        Ok((vm.items.math_atom)(self.get().into()))
    }
}

impl Eval for ast::Symbol {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.symbol)(self.get().into()))
    }
}

impl ast::Symbol {
    fn eval_in_math(&self, vm: &mut Vm) -> SourceResult<Content> {
        Ok((vm.items.symbol)(EcoString::from(self.get()) + ":op".into()))
    }
}

impl Eval for ast::SmartQuote {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.smart_quote)(self.double()))
    }
}

impl Eval for ast::Strong {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.strong)(self.body().eval(vm)?))
    }
}

impl Eval for ast::Emph {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.emph)(self.body().eval(vm)?))
    }
}

impl Eval for ast::Raw {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let text = self.text();
        let lang = self.lang().map(Into::into);
        let block = self.block();
        Ok((vm.items.raw)(text, lang, block))
    }
}

impl Eval for ast::Link {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.link)(self.url().clone()))
    }
}

impl Eval for ast::Label {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Label(Label(self.get().into())))
    }
}

impl Eval for ast::Ref {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.ref_)(self.get().into()))
    }
}

impl Eval for ast::Heading {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let level = self.level();
        let body = self.body().eval(vm)?;
        Ok((vm.items.heading)(level, body))
    }
}

impl Eval for ast::ListItem {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.list_item)(self.body().eval(vm)?))
    }
}

impl Eval for ast::EnumItem {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let number = self.number();
        let body = self.body().eval(vm)?;
        Ok((vm.items.enum_item)(number, body))
    }
}

impl Eval for ast::TermItem {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let term = self.term().eval(vm)?;
        let description = self.description().eval(vm)?;
        Ok((vm.items.term_item)(term, description))
    }
}

impl Eval for ast::Math {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let seq = self
            .exprs()
            .map(|expr| expr.eval_in_math(vm))
            .collect::<SourceResult<_>>()?;
        let block = self.block();
        Ok((vm.items.math)(seq, block))
    }
}

impl Eval for ast::Atom {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.math_atom)(self.get().clone()))
    }
}

impl Eval for ast::Script {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let base = self.base().eval_in_math(vm)?;
        let sub = self.sub().map(|expr| expr.eval_in_math(vm)).transpose()?;
        let sup = self.sup().map(|expr| expr.eval_in_math(vm)).transpose()?;
        Ok((vm.items.math_script)(base, sub, sup))
    }
}

impl Eval for ast::Frac {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let num = self.num().eval_in_math(vm)?;
        let denom = self.denom().eval_in_math(vm)?;
        Ok((vm.items.math_frac)(num, denom))
    }
}

impl Eval for ast::AlignPoint {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok((vm.items.math_align_point)())
    }
}

impl Eval for ast::Ident {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = vm.scopes.get(self).cloned().at(self.span())?;
        Ok(match value {
            Value::Func(func) => Value::Func(func.spanned(self.span())),
            value => value,
        })
    }
}

impl ast::Ident {
    fn eval_in_math(&self, vm: &mut Vm) -> SourceResult<Content> {
        if self.as_untyped().len() == self.len()
            && matches!(vm.scopes.get(&self), Ok(Value::Func(_)) | Err(_))
        {
            Ok((vm.items.symbol)(EcoString::from(self.get()) + ":op".into()))
        } else {
            Ok(self.eval(vm)?.display_in_math())
        }
    }
}

impl Eval for ast::None {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::None)
    }
}

impl Eval for ast::Auto {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Auto)
    }
}

impl Eval for ast::Bool {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Bool(self.get()))
    }
}

impl Eval for ast::Int {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Int(self.get()))
    }
}

impl Eval for ast::Float {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Float(self.get()))
    }
}

impl Eval for ast::Numeric {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        let (v, unit) = self.get();
        Ok(match unit {
            ast::Unit::Length(unit) => Abs::with_unit(v, unit).into(),
            ast::Unit::Angle(unit) => Angle::with_unit(v, unit).into(),
            ast::Unit::Em => Em::new(v).into(),
            ast::Unit::Fr => Fr::new(v).into(),
            ast::Unit::Percent => Ratio::new(v / 100.0).into(),
        })
    }
}

impl Eval for ast::Str {
    type Output = Value;

    fn eval(&self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Str(self.get().clone().into()))
    }
}

impl Eval for ast::CodeBlock {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.enter();
        let output = eval_code(vm, &mut self.exprs())?;
        vm.scopes.exit();
        Ok(output)
    }
}

/// Evaluate a stream of expressions.
fn eval_code(
    vm: &mut Vm,
    exprs: &mut impl Iterator<Item = ast::Expr>,
) -> SourceResult<Value> {
    let flow = vm.flow.take();
    let mut output = Value::None;

    while let Some(expr) = exprs.next() {
        let span = expr.span();
        let value = match expr {
            ast::Expr::Set(set) => {
                let styles = set.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                let tail = eval_code(vm, exprs)?.display();
                Value::Content(tail.styled_with_map(styles))
            }
            ast::Expr::Show(show) => {
                let recipe = show.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                let tail = eval_code(vm, exprs)?.display();
                Value::Content(tail.styled_with_recipe(vm.world, recipe)?)
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

impl Eval for ast::ContentBlock {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.enter();
        let content = self.body().eval(vm)?;
        vm.scopes.exit();
        Ok(content)
    }
}

impl Eval for ast::Parenthesized {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        self.expr().eval(vm)
    }
}

impl Eval for ast::Array {
    type Output = Array;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let items = self.items();

        let mut vec = Vec::with_capacity(items.size_hint().0);
        for item in items {
            match item {
                ast::ArrayItem::Pos(expr) => vec.push(expr.eval(vm)?),
                ast::ArrayItem::Spread(expr) => match expr.eval(vm)? {
                    Value::None => {}
                    Value::Array(array) => vec.extend(array.into_iter()),
                    v => bail!(expr.span(), "cannot spread {} into array", v.type_name()),
                },
            }
        }

        Ok(Array::from_vec(vec))
    }
}

impl Eval for ast::Dict {
    type Output = Dict;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let mut map = BTreeMap::new();

        for item in self.items() {
            match item {
                ast::DictItem::Named(named) => {
                    map.insert(named.name().take().into(), named.expr().eval(vm)?);
                }
                ast::DictItem::Keyed(keyed) => {
                    map.insert(keyed.key().get().into(), keyed.expr().eval(vm)?);
                }
                ast::DictItem::Spread(expr) => match expr.eval(vm)? {
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

impl Eval for ast::Unary {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let value = self.expr().eval(vm)?;
        let result = match self.op() {
            ast::UnOp::Pos => ops::pos(value),
            ast::UnOp::Neg => ops::neg(value),
            ast::UnOp::Not => ops::not(value),
        };
        result.at(self.span())
    }
}

impl Eval for ast::Binary {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        match self.op() {
            ast::BinOp::Add => self.apply(vm, ops::add),
            ast::BinOp::Sub => self.apply(vm, ops::sub),
            ast::BinOp::Mul => self.apply(vm, ops::mul),
            ast::BinOp::Div => self.apply(vm, ops::div),
            ast::BinOp::And => self.apply(vm, ops::and),
            ast::BinOp::Or => self.apply(vm, ops::or),
            ast::BinOp::Eq => self.apply(vm, ops::eq),
            ast::BinOp::Neq => self.apply(vm, ops::neq),
            ast::BinOp::Lt => self.apply(vm, ops::lt),
            ast::BinOp::Leq => self.apply(vm, ops::leq),
            ast::BinOp::Gt => self.apply(vm, ops::gt),
            ast::BinOp::Geq => self.apply(vm, ops::geq),
            ast::BinOp::In => self.apply(vm, ops::in_),
            ast::BinOp::NotIn => self.apply(vm, ops::not_in),
            ast::BinOp::Assign => self.assign(vm, |_, b| Ok(b)),
            ast::BinOp::AddAssign => self.assign(vm, ops::add),
            ast::BinOp::SubAssign => self.assign(vm, ops::sub),
            ast::BinOp::MulAssign => self.assign(vm, ops::mul),
            ast::BinOp::DivAssign => self.assign(vm, ops::div),
        }
    }
}

impl ast::Binary {
    /// Apply a basic binary operation.
    fn apply(
        &self,
        vm: &mut Vm,
        op: fn(Value, Value) -> StrResult<Value>,
    ) -> SourceResult<Value> {
        let lhs = self.lhs().eval(vm)?;

        // Short-circuit boolean operations.
        if (self.op() == ast::BinOp::And && lhs == Value::Bool(false))
            || (self.op() == ast::BinOp::Or && lhs == Value::Bool(true))
        {
            return Ok(lhs);
        }

        let rhs = self.rhs().eval(vm)?;
        op(lhs, rhs).at(self.span())
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

impl Eval for ast::FieldAccess {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let object = self.target().eval(vm)?;
        let span = self.field().span();
        let field = self.field().take();

        Ok(match object {
            Value::Dict(dict) => dict.at(&field).at(span)?.clone(),
            Value::Content(content) => content
                .field(&field)
                .ok_or_else(|| format!("unknown field `{field}`"))
                .at(span)?,
            Value::Module(module) => module
                .scope()
                .get(&field)
                .cloned()
                .ok_or_else(|| {
                    format!("module `{}` does not contain `{field}`", module.name())
                })
                .at(span)?,
            v => bail!(
                self.target().span(),
                "expected dictionary or content, found {}",
                v.type_name()
            ),
        })
    }
}

impl Eval for ast::FuncCall {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.depth >= MAX_CALL_DEPTH {
            bail!(self.span(), "maximum function call depth exceeded");
        }

        let callee = self.callee();
        let callee = callee.eval(vm)?.cast::<Func>().at(callee.span())?;
        let args = self.args().eval(vm)?;
        let point = || Tracepoint::Call(callee.name().map(Into::into));
        callee.call(vm, args).trace(vm.world, point, self.span())
    }
}

impl Eval for ast::MethodCall {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.span();
        let method = self.method().take();

        let result = if methods::is_mutating(&method) {
            let args = self.args().eval(vm)?;
            let value = self.target().access(vm)?;
            methods::call_mut(value, &method, args, span)
        } else {
            let value = self.target().eval(vm)?;
            let args = self.args().eval(vm)?;
            methods::call(vm, value, &method, args, span)
        };

        let point = || Tracepoint::Call(Some(method.clone()));
        result.trace(vm.world, point, span)
    }
}

impl Eval for ast::Args {
    type Output = Args;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let mut items = Vec::new();

        for arg in self.items() {
            let span = arg.span();
            match arg {
                ast::Arg::Pos(expr) => {
                    items.push(Arg {
                        span,
                        name: None,
                        value: Spanned::new(expr.eval(vm)?, expr.span()),
                    });
                }
                ast::Arg::Named(named) => {
                    items.push(Arg {
                        span,
                        name: Some(named.name().take().into()),
                        value: Spanned::new(named.expr().eval(vm)?, named.expr().span()),
                    });
                }
                ast::Arg::Spread(expr) => match expr.eval(vm)? {
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

impl Eval for ast::Closure {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        // The closure's name is defined by its let binding if there's one.
        let name = self.name().map(ast::Ident::take);

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
                ast::Param::Pos(name) => {
                    params.push((name.take(), None));
                }
                ast::Param::Named(named) => {
                    params.push((named.name().take(), Some(named.expr().eval(vm)?)));
                }
                ast::Param::Sink(name) => {
                    if sink.is_some() {
                        bail!(name.span(), "only one argument sink is allowed");
                    }
                    sink = Some(name.take());
                }
            }
        }

        // Define the closure.
        let closure = Closure {
            location: vm.location,
            name,
            captured,
            params,
            sink,
            body: self.body(),
        };

        Ok(Value::Func(Func::from_closure(closure, self.span())))
    }
}

impl Eval for ast::LetBinding {
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

impl Eval for ast::SetRule {
    type Output = StyleMap;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if let Some(condition) = self.condition() {
            if !condition.eval(vm)?.cast::<bool>().at(condition.span())? {
                return Ok(StyleMap::new());
            }
        }

        let target = self.target();
        let target = target.eval(vm)?.cast::<Func>().at(target.span())?;
        let args = self.args().eval(vm)?;
        Ok(target.set(args)?.spanned(self.span()))
    }
}

impl Eval for ast::ShowRule {
    type Output = Recipe;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let selector = self
            .selector()
            .map(|sel| sel.eval(vm)?.cast::<Selector>().at(sel.span()))
            .transpose()?;

        let transform = self.transform();
        let span = transform.span();

        let transform = match transform {
            ast::Expr::Set(set) => Transform::Style(set.eval(vm)?),
            expr => expr.eval(vm)?.cast::<Transform>().at(span)?,
        };

        Ok(Recipe { span, selector, transform })
    }
}

impl Eval for ast::Conditional {
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

impl Eval for ast::WhileLoop {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let flow = vm.flow.take();
        let mut output = Value::None;
        let mut i = 0;

        let condition = self.condition();
        let body = self.body();

        while condition.eval(vm)?.cast::<bool>().at(condition.span())? {
            if i == 0
                && is_invariant(condition.as_untyped())
                && !can_diverge(body.as_untyped())
            {
                bail!(condition.span(), "condition is always true");
            } else if i >= MAX_ITERATIONS {
                bail!(self.span(), "loop seems to be infinite");
            }

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

            i += 1;
        }

        if flow.is_some() {
            vm.flow = flow;
        }

        Ok(output)
    }
}

/// Whether the expression always evaluates to the same value.
fn is_invariant(expr: &SyntaxNode) -> bool {
    match expr.cast() {
        Some(ast::Expr::Ident(_)) => false,
        Some(ast::Expr::MethodCall(call)) => {
            is_invariant(call.target().as_untyped())
                && is_invariant(call.args().as_untyped())
        }
        _ => expr.children().all(is_invariant),
    }
}

/// Whether the expression contains a break or return.
fn can_diverge(expr: &SyntaxNode) -> bool {
    matches!(expr.kind(), SyntaxKind::Break | SyntaxKind::Return)
        || expr.children().any(can_diverge)
}

impl Eval for ast::ForLoop {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let flow = vm.flow.take();
        let mut output = Value::None;

        macro_rules! iter {
            (for ($($binding:ident => $value:ident),*) in $iter:expr) => {{
                vm.scopes.enter();

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

                vm.scopes.exit();
            }};
        }

        let iter = self.iter().eval(vm)?;
        let pattern = self.pattern();
        let key = pattern.key().map(ast::Ident::take);
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

        Ok(output)
    }
}

impl Eval for ast::ModuleImport {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.source().span();
        let source = self.source().eval(vm)?;
        let module = import(vm, source, span)?;

        match self.imports() {
            None => {
                vm.scopes.top.define(module.name().clone(), module);
            }
            Some(ast::Imports::Wildcard) => {
                for (var, value) in module.scope().iter() {
                    vm.scopes.top.define(var.clone(), value.clone());
                }
            }
            Some(ast::Imports::Items(idents)) => {
                for ident in idents {
                    if let Some(value) = module.scope().get(&ident) {
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

impl Eval for ast::ModuleInclude {
    type Output = Content;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let span = self.source().span();
        let source = self.source().eval(vm)?;
        let module = import(vm, source, span)?;
        Ok(module.content())
    }
}

/// Process an import of a module relative to the current location.
fn import(vm: &Vm, source: Value, span: Span) -> SourceResult<Module> {
    let path = match source {
        Value::Str(path) => path,
        Value::Module(module) => return Ok(module),
        v => bail!(span, "expected path or module, found {}", v.type_name()),
    };

    // Load the source file.
    let full = vm.locate(&path).at(span)?;
    let id = vm.world.resolve(&full).at(span)?;

    // Prevent cyclic importing.
    if vm.route.contains(id) {
        bail!(span, "cyclic import");
    }

    // Evaluate the file.
    let source = vm.world.source(id);
    let point = || Tracepoint::Import;
    eval(vm.world, vm.route, source).trace(vm.world, point, span)
}

impl Eval for ast::LoopBreak {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.flow.is_none() {
            vm.flow = Some(Flow::Break(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ast::LoopContinue {
    type Output = Value;

    fn eval(&self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if vm.flow.is_none() {
            vm.flow = Some(Flow::Continue(self.span()));
        }
        Ok(Value::None)
    }
}

impl Eval for ast::FuncReturn {
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
trait Access {
    /// Access the value.
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value>;
}

impl Access for ast::Expr {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        match self {
            Self::Ident(v) => v.access(vm),
            Self::Parenthesized(v) => v.access(vm),
            Self::FieldAccess(v) => v.access(vm),
            Self::MethodCall(v) => v.access(vm),
            _ => {
                let _ = self.eval(vm)?;
                bail!(self.span(), "cannot mutate a temporary value");
            }
        }
    }
}

impl Access for ast::Ident {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        vm.scopes.get_mut(self).at(self.span())
    }
}

impl Access for ast::Parenthesized {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        self.expr().access(vm)
    }
}

impl Access for ast::FieldAccess {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        let value = self.target().access(vm)?;
        let Value::Dict(dict) = value else {
            bail!(
                self.target().span(),
                "expected dictionary, found {}",
                value.type_name(),
            );
        };

        Ok(dict.at_mut(self.field().take().into()))
    }
}

impl Access for ast::MethodCall {
    fn access<'a>(&self, vm: &'a mut Vm) -> SourceResult<&'a mut Value> {
        let span = self.span();
        let method = self.method().take();
        let world = vm.world();

        if !methods::is_accessor(&method) {
            let _ = self.eval(vm)?;
            bail!(span, "cannot mutate a temporary value");
        }

        let args = self.args().eval(vm)?;
        let value = self.target().access(vm)?;
        let result = methods::call_access(value, &method, args, span);

        let point = || Tracepoint::Call(Some(method.clone()));
        result.trace(world, point, span)
    }
}

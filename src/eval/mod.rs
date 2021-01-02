//! Evaluation of syntax trees.

#[macro_use]
mod value;
mod args;
mod scope;
mod state;

pub use args::*;
pub use scope::*;
pub use state::*;
pub use value::*;

use std::any::Any;
use std::rc::Rc;

use fontdock::FontStyle;

use crate::color::Color;
use crate::diag::Diag;
use crate::diag::{Deco, Feedback, Pass};
use crate::env::SharedEnv;
use crate::geom::{BoxAlign, Dir, Flow, Gen, Length, Linear, Relative, Sides, Size};
use crate::layout::{
    Document, Expansion, LayoutNode, Pad, Pages, Par, Spacing, Stack, Text,
};
use crate::syntax::*;

/// Evaluate a syntax tree into a document.
///
/// The given `state` is the base state that may be updated over the course of
/// evaluation.
pub fn eval(tree: &SynTree, env: SharedEnv, state: State) -> Pass<Document> {
    let mut ctx = EvalContext::new(env, state);
    ctx.start_page_group(Softness::Hard);
    tree.eval(&mut ctx);
    ctx.end_page_group(|s| s == Softness::Hard);
    ctx.finish()
}

/// The context for evaluation.
#[derive(Debug)]
pub struct EvalContext {
    /// The environment from which resources are gathered.
    pub env: SharedEnv,
    /// The active evaluation state.
    pub state: State,
    /// The accumulated feedback.
    feedback: Feedback,
    /// The finished page runs.
    runs: Vec<Pages>,
    /// The stack of logical groups (paragraphs and such).
    ///
    /// Each entry contains metadata about the group and nodes that are at the
    /// same level as the group, which will return to `inner` once the group is
    /// finished.
    groups: Vec<(Box<dyn Any>, Vec<LayoutNode>)>,
    /// The nodes in the current innermost group
    /// (whose metadata is in `groups.last()`).
    inner: Vec<LayoutNode>,
}

impl EvalContext {
    /// Create a new evaluation context with a base state.
    pub fn new(env: SharedEnv, state: State) -> Self {
        Self {
            env,
            state,
            groups: vec![],
            inner: vec![],
            runs: vec![],
            feedback: Feedback::new(),
        }
    }

    /// Finish evaluation and return the created document.
    pub fn finish(self) -> Pass<Document> {
        assert!(self.groups.is_empty(), "unfinished group");
        Pass::new(Document { runs: self.runs }, self.feedback)
    }

    /// Add a diagnostic to the feedback.
    pub fn diag(&mut self, diag: Spanned<Diag>) {
        self.feedback.diags.push(diag);
    }

    /// Add a decoration to the feedback.
    pub fn deco(&mut self, deco: Spanned<Deco>) {
        self.feedback.decos.push(deco);
    }

    /// Push a layout node to the active group.
    ///
    /// Spacing nodes will be handled according to their [`Softness`].
    pub fn push(&mut self, node: impl Into<LayoutNode>) {
        let node = node.into();

        if let LayoutNode::Spacing(this) = node {
            if this.softness == Softness::Soft && self.inner.is_empty() {
                return;
            }

            if let Some(&LayoutNode::Spacing(other)) = self.inner.last() {
                if this.softness > other.softness {
                    self.inner.pop();
                } else if this.softness == Softness::Soft {
                    return;
                }
            }
        }

        self.inner.push(node);
    }

    /// Start a page group based on the active page state.
    ///
    /// The `softness` is a hint on whether empty pages should be kept in the
    /// output.
    ///
    /// This also starts an inner paragraph.
    pub fn start_page_group(&mut self, softness: Softness) {
        self.start_group(PageGroup {
            size: self.state.page.size,
            padding: self.state.page.margins(),
            flow: self.state.flow,
            align: self.state.align,
            softness,
        });
        self.start_par_group();
    }

    /// End a page group, returning its [`Softness`].
    ///
    /// Whether the page is kept when it's empty is decided by `keep_empty`
    /// based on its softness. If kept, the page is pushed to the finished page
    /// runs.
    ///
    /// This also ends an inner paragraph.
    pub fn end_page_group(
        &mut self,
        keep_empty: impl FnOnce(Softness) -> bool,
    ) -> Softness {
        self.end_par_group();
        let (group, children) = self.end_group::<PageGroup>();
        if !children.is_empty() || keep_empty(group.softness) {
            self.runs.push(Pages {
                size: group.size,
                child: LayoutNode::dynamic(Pad {
                    padding: group.padding,
                    child: LayoutNode::dynamic(Stack {
                        flow: group.flow,
                        align: group.align,
                        expansion: Gen::uniform(Expansion::Fill),
                        children,
                    }),
                }),
            })
        }
        group.softness
    }

    /// Start a content group.
    ///
    /// This also starts an inner paragraph.
    pub fn start_content_group(&mut self) {
        self.start_group(ContentGroup);
        self.start_par_group();
    }

    /// End a content group and return the resulting nodes.
    ///
    /// This also ends an inner paragraph.
    pub fn end_content_group(&mut self) -> Vec<LayoutNode> {
        self.end_par_group();
        self.end_group::<ContentGroup>().1
    }

    /// Start a paragraph group based on the active text state.
    pub fn start_par_group(&mut self) {
        let em = self.state.font.font_size();
        self.start_group(ParGroup {
            flow: self.state.flow,
            align: self.state.align,
            line_spacing: self.state.par.line_spacing.resolve(em),
        });
    }

    /// End a paragraph group and push it to its parent group if it's not empty.
    pub fn end_par_group(&mut self) {
        let (group, children) = self.end_group::<ParGroup>();
        if !children.is_empty() {
            // FIXME: This is a hack and should be superseded by something
            //        better.
            let cross_expansion = Expansion::fill_if(self.groups.len() <= 1);
            self.push(Par {
                flow: group.flow,
                align: group.align,
                cross_expansion,
                line_spacing: group.line_spacing,
                children,
            });
        }
    }

    /// Start a layouting group.
    ///
    /// All further calls to [`push`](Self::push) will collect nodes for this group.
    /// The given metadata will be returned alongside the collected nodes
    /// in a matching call to [`end_group`](Self::end_group).
    fn start_group<T: 'static>(&mut self, meta: T) {
        self.groups.push((Box::new(meta), std::mem::take(&mut self.inner)));
    }

    /// End a layouting group started with [`start_group`](Self::start_group).
    ///
    /// This returns the stored metadata and the collected nodes.
    #[track_caller]
    fn end_group<T: 'static>(&mut self) -> (T, Vec<LayoutNode>) {
        if let Some(&LayoutNode::Spacing(spacing)) = self.inner.last() {
            if spacing.softness == Softness::Soft {
                self.inner.pop();
            }
        }

        let (any, outer) = self.groups.pop().expect("no pushed group");
        let group = *any.downcast::<T>().expect("bad group type");
        (group, std::mem::replace(&mut self.inner, outer))
    }

    /// Updates the flow directions if the resulting main and cross directions
    /// apply to different axes. Generates an appropriate error, otherwise.
    pub fn set_flow(&mut self, new: Gen<Option<Spanned<Dir>>>) {
        let flow = Gen::new(
            new.main.map(|s| s.v).unwrap_or(self.state.flow.main),
            new.cross.map(|s| s.v).unwrap_or(self.state.flow.cross),
        );

        if flow.main.axis() != flow.cross.axis() {
            self.state.flow = flow;
        } else {
            for dir in new.main.iter().chain(new.cross.iter()) {
                self.diag(error!(dir.span, "aligned axis"));
            }
        }
    }

    /// Apply a forced line break.
    pub fn apply_linebreak(&mut self) {
        self.end_par_group();
        self.start_par_group();
    }

    /// Apply a forced paragraph break.
    pub fn apply_parbreak(&mut self) {
        self.end_par_group();
        let em = self.state.font.font_size();
        self.push(Spacing {
            amount: self.state.par.par_spacing.resolve(em),
            softness: Softness::Soft,
        });
        self.start_par_group();
    }

    /// Construct a text node from the given string based on the active text
    /// state.
    pub fn make_text_node(&self, text: String) -> Text {
        let mut variant = self.state.font.variant;

        if self.state.font.strong {
            variant.weight = variant.weight.thicken(300);
        }

        if self.state.font.emph {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        Text {
            text,
            align: self.state.align,
            dir: self.state.flow.cross,
            font_size: self.state.font.font_size(),
            families: Rc::clone(&self.state.font.families),
            variant,
        }
    }
}

/// A group for page runs.
struct PageGroup {
    size: Size,
    padding: Sides<Linear>,
    flow: Flow,
    align: BoxAlign,
    softness: Softness,
}

/// A group for generic content.
struct ContentGroup;

/// A group for paragraphs.
struct ParGroup {
    flow: Flow,
    align: BoxAlign,
    line_spacing: Length,
}

/// Defines how an item interact with surrounding items.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Softness {
    /// A soft item can be skipped in some circumstances.
    Soft,
    /// A hard item is always retained.
    Hard,
}

/// Evaluate an item.
///
/// _Note_: Evaluation is not necessarily pure, it may change the active state.
pub trait Eval {
    /// The output of evaluating the item.
    type Output;

    /// Evaluate the item to the output value.
    fn eval(self, ctx: &mut EvalContext) -> Self::Output;
}

impl<'a, T> Eval for &'a Box<Spanned<T>>
where
    Spanned<&'a T>: Eval,
{
    type Output = <Spanned<&'a T> as Eval>::Output;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        (**self).as_ref().eval(ctx)
    }
}

impl Eval for &[Spanned<SynNode>] {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        for node in self {
            node.as_ref().eval(ctx);
        }
    }
}

impl Eval for Spanned<&SynNode> {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match self.v {
            SynNode::Text(text) => {
                let node = ctx.make_text_node(text.clone());
                ctx.push(node);
            }

            SynNode::Space => {
                let em = ctx.state.font.font_size();
                ctx.push(Spacing {
                    amount: ctx.state.par.word_spacing.resolve(em),
                    softness: Softness::Soft,
                });
            }
            SynNode::Linebreak => ctx.apply_linebreak(),
            SynNode::Parbreak => ctx.apply_parbreak(),

            SynNode::Strong => ctx.state.font.strong ^= true,
            SynNode::Emph => ctx.state.font.emph ^= true,

            SynNode::Heading(heading) => heading.with_span(self.span).eval(ctx),
            SynNode::Raw(raw) => raw.with_span(self.span).eval(ctx),

            SynNode::Expr(expr) => {
                let value = expr.with_span(self.span).eval(ctx);
                value.eval(ctx)
            }
        }
    }
}

impl Eval for Spanned<&NodeHeading> {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let prev = ctx.state.clone();
        let upscale = 1.5 - 0.1 * self.v.level.v as f64;
        ctx.state.font.scale *= upscale;
        ctx.state.font.strong = true;

        self.v.contents.eval(ctx);
        ctx.apply_parbreak();

        ctx.state = prev;
    }
}

impl Eval for Spanned<&NodeRaw> {
    type Output = ();

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let prev = Rc::clone(&ctx.state.font.families);
        let families = Rc::make_mut(&mut ctx.state.font.families);
        families.list.insert(0, "monospace".to_string());
        families.flatten();

        let em = ctx.state.font.font_size();
        let line_spacing = ctx.state.par.line_spacing.resolve(em);

        let mut children = vec![];
        for line in &self.v.lines {
            children.push(LayoutNode::Text(ctx.make_text_node(line.clone())));
            children.push(LayoutNode::Spacing(Spacing {
                amount: line_spacing,
                softness: Softness::Hard,
            }));
        }

        ctx.push(Stack {
            flow: ctx.state.flow,
            align: ctx.state.align,
            expansion: Gen::uniform(Expansion::Fit),
            children,
        });

        ctx.state.font.families = prev;
    }
}

impl Eval for Spanned<&Expr> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match self.v {
            Expr::Lit(lit) => lit.with_span(self.span).eval(ctx),
            Expr::Call(call) => call.with_span(self.span).eval(ctx),
            Expr::Unary(unary) => unary.with_span(self.span).eval(ctx),
            Expr::Binary(binary) => binary.with_span(self.span).eval(ctx),
        }
    }
}

impl Eval for Spanned<&Lit> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        match *self.v {
            Lit::Ident(ref v) => match ctx.state.scope.get(v.as_str()) {
                Some(value) => value.clone(),
                None => {
                    ctx.diag(error!(self.span, "unknown variable"));
                    Value::Error
                }
            },
            Lit::Bool(v) => Value::Bool(v),
            Lit::Int(v) => Value::Int(v),
            Lit::Float(v) => Value::Float(v),
            Lit::Length(v, unit) => Value::Length(Length::with_unit(v, unit)),
            Lit::Percent(v) => Value::Relative(Relative::new(v / 100.0)),
            Lit::Color(v) => Value::Color(Color::Rgba(v)),
            Lit::Str(ref v) => Value::Str(v.clone()),
            Lit::Array(ref v) => Value::Array(v.with_span(self.span).eval(ctx)),
            Lit::Dict(ref v) => Value::Dict(v.with_span(self.span).eval(ctx)),
            Lit::Content(ref v) => Value::Content(v.clone()),
        }
    }
}

impl Eval for Spanned<&Array> {
    type Output = ValueArray;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.v.iter().map(|expr| expr.as_ref().eval(ctx)).collect()
    }
}

impl Eval for Spanned<&Dict> {
    type Output = ValueDict;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        self.v
            .iter()
            .map(|Named { name, expr }| (name.v.0.clone(), expr.as_ref().eval(ctx)))
            .collect()
    }
}

impl Eval for Spanned<&ExprCall> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let name = &self.v.name.v;
        let span = self.v.name.span;

        if let Some(value) = ctx.state.scope.get(name) {
            if let Value::Func(func) = value {
                let func = func.clone();
                ctx.feedback.decos.push(Deco::Resolved.with_span(span));

                let mut args = self.v.args.as_ref().eval(ctx);
                let returned = func(ctx, &mut args);
                args.finish(ctx);

                return returned;
            } else {
                let ty = value.type_name();
                ctx.diag(error!(span, "a value of type {} is not callable", ty));
            }
        } else if !name.is_empty() {
            ctx.diag(error!(span, "unknown function"));
        }

        ctx.feedback.decos.push(Deco::Unresolved.with_span(span));
        Value::Error
    }
}

impl Eval for Spanned<&ExprUnary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let value = self.v.expr.eval(ctx);

        if let Value::Error = value {
            return Value::Error;
        }

        let span = self.v.op.span.join(self.v.expr.span);
        match self.v.op.v {
            UnOp::Neg => neg(ctx, span, value),
        }
    }
}

impl Eval for Spanned<&ExprBinary> {
    type Output = Value;

    fn eval(self, ctx: &mut EvalContext) -> Self::Output {
        let lhs = self.v.lhs.eval(ctx);
        let rhs = self.v.rhs.eval(ctx);

        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        let span = self.v.lhs.span.join(self.v.rhs.span);
        match self.v.op.v {
            BinOp::Add => add(ctx, span, lhs, rhs),
            BinOp::Sub => sub(ctx, span, lhs, rhs),
            BinOp::Mul => mul(ctx, span, lhs, rhs),
            BinOp::Div => div(ctx, span, lhs, rhs),
        }
    }
}

/// Compute the negation of a value.
fn neg(ctx: &mut EvalContext, span: Span, value: Value) -> Value {
    use Value::*;
    match value {
        Int(v) => Int(-v),
        Float(v) => Float(-v),
        Length(v) => Length(-v),
        Relative(v) => Relative(-v),
        Linear(v) => Linear(-v),
        v => {
            ctx.diag(error!(span, "cannot negate {}", v.type_name()));
            Value::Error
        }
    }
}

/// Compute the sum of two values.
fn add(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers to themselves.
        (Int(a), Int(b)) => Int(a + b),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
        (Float(a), Float(b)) => Float(a + b),

        // Lengths, relatives and linears to themselves.
        (Length(a), Length(b)) => Length(a + b),
        (Length(a), Relative(b)) => Linear(a + b),
        (Length(a), Linear(b)) => Linear(a + b),

        (Relative(a), Length(b)) => Linear(a + b),
        (Relative(a), Relative(b)) => Relative(a + b),
        (Relative(a), Linear(b)) => Linear(a + b),

        (Linear(a), Length(b)) => Linear(a + b),
        (Linear(a), Relative(b)) => Linear(a + b),
        (Linear(a), Linear(b)) => Linear(a + b),

        // Complex data types to themselves.
        (Str(a), Str(b)) => Str(a + &b),
        (Dict(a), Dict(b)) => Dict(concat(a, b)),
        (Content(a), Content(b)) => Content(concat(a, b)),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot add {} and {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the difference of two values.
fn sub(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers from themselves.
        (Int(a), Int(b)) => Int(a - b),
        (Int(a), Float(b)) => Float(a as f64 - b),
        (Float(a), Int(b)) => Float(a - b as f64),
        (Float(a), Float(b)) => Float(a - b),

        // Lengths, relatives and linears from themselves.
        (Length(a), Length(b)) => Length(a - b),
        (Length(a), Relative(b)) => Linear(a - b),
        (Length(a), Linear(b)) => Linear(a - b),
        (Relative(a), Length(b)) => Linear(a - b),
        (Relative(a), Relative(b)) => Relative(a - b),
        (Relative(a), Linear(b)) => Linear(a - b),
        (Linear(a), Length(b)) => Linear(a - b),
        (Linear(a), Relative(b)) => Linear(a - b),
        (Linear(a), Linear(b)) => Linear(a - b),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot subtract {1} from {0}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the product of two values.
fn mul(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers with themselves.
        (Int(a), Int(b)) => Int(a * b),
        (Int(a), Float(b)) => Float(a as f64 * b),
        (Float(a), Int(b)) => Float(a * b as f64),
        (Float(a), Float(b)) => Float(a * b),

        // Lengths, relatives and linears with numbers.
        (Length(a), Int(b)) => Length(a * b as f64),
        (Length(a), Float(b)) => Length(a * b),
        (Int(a), Length(b)) => Length(a as f64 * b),
        (Float(a), Length(b)) => Length(a * b),
        (Relative(a), Int(b)) => Relative(a * b as f64),
        (Relative(a), Float(b)) => Relative(a * b),
        (Int(a), Relative(b)) => Relative(a as f64 * b),
        (Float(a), Relative(b)) => Relative(a * b),
        (Linear(a), Int(b)) => Linear(a * b as f64),
        (Linear(a), Float(b)) => Linear(a * b),
        (Int(a), Linear(b)) => Linear(a as f64 * b),
        (Float(a), Linear(b)) => Linear(a * b),

        // Integers with strings.
        (Int(a), Str(b)) => Str(b.repeat(0.max(a) as usize)),
        (Str(a), Int(b)) => Str(a.repeat(0.max(b) as usize)),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot multiply {} with {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Compute the quotient of two values.
fn div(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use Value::*;
    match (lhs, rhs) {
        // Numbers by themselves.
        (Int(a), Int(b)) => Float(a as f64 / b as f64),
        (Int(a), Float(b)) => Float(a as f64 / b),
        (Float(a), Int(b)) => Float(a / b as f64),
        (Float(a), Float(b)) => Float(a / b),

        // Lengths by numbers.
        (Length(a), Int(b)) => Length(a / b as f64),
        (Length(a), Float(b)) => Length(a / b),
        (Relative(a), Int(b)) => Relative(a / b as f64),
        (Relative(a), Float(b)) => Relative(a / b),
        (Linear(a), Int(b)) => Linear(a / b as f64),
        (Linear(a), Float(b)) => Linear(a / b),

        (a, b) => {
            ctx.diag(error!(
                span,
                "cannot divide {} by {}",
                a.type_name(),
                b.type_name()
            ));
            Value::Error
        }
    }
}

/// Concatenate two collections.
fn concat<T, A>(mut a: T, b: T) -> T
where
    T: Extend<A> + IntoIterator<Item = A>,
{
    a.extend(b);
    a
}

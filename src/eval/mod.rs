//! Evaluation of syntax trees.

mod args;
mod convert;
mod dict;
mod scope;
mod state;
mod value;

pub use args::*;
pub use convert::*;
pub use dict::*;
pub use scope::*;
pub use state::*;
pub use value::*;

use std::any::Any;
use std::mem;
use std::rc::Rc;

use fontdock::FontStyle;

use crate::diag::Diag;
use crate::diag::{Deco, Feedback, Pass};
use crate::layout::nodes::{
    Document, LayoutNode, Pad, Pages, Par, Softness, Spacing, Stack, Text,
};
use crate::layout::{Gen2, Spec2, Switch};
use crate::syntax::*;

/// Evaluate a syntax tree into a document.
///
/// The given `state` the base state that may be updated over the course of
/// evaluation.
pub fn eval(tree: &SynTree, state: State) -> Pass<Document> {
    let mut ctx = EvalContext::new(state);

    ctx.start_page_group(false);
    tree.eval(&mut ctx);
    ctx.end_page_group();

    ctx.finish()
}

/// The context for evaluation.
#[derive(Debug)]
pub struct EvalContext {
    /// The active evaluation state.
    pub state: State,
    /// The accumualted feedback.
    f: Feedback,
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
    pub fn new(state: State) -> Self {
        Self {
            state,
            groups: vec![],
            inner: vec![],
            runs: vec![],
            f: Feedback::new(),
        }
    }

    /// Finish evaluation and return the created document.
    pub fn finish(self) -> Pass<Document> {
        assert!(self.groups.is_empty(), "unpoped group");
        Pass::new(Document { runs: self.runs }, self.f)
    }

    /// Add a diagnostic to the feedback.
    pub fn diag(&mut self, diag: Spanned<Diag>) {
        self.f.diags.push(diag);
    }

    /// Add a decoration to the feedback.
    pub fn deco(&mut self, deco: Spanned<Deco>) {
        self.f.decos.push(deco);
    }

    /// Push a layout node to the active group.
    ///
    /// Spacing nodes will be handled according to their [`Softness`].
    ///
    /// [`Softness`]: ../layout/nodes/enum.Softness.html
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

    /// Start a layouting group.
    ///
    /// All further calls to [`push`] will collect nodes for this group.
    /// The given metadata will be returned alongside the collected nodes
    /// in a matching call to [`end_group`].
    ///
    /// [`push`]: #method.push
    /// [`end_group`]: #method.end_group
    pub fn start_group<T: 'static>(&mut self, meta: T) {
        self.groups.push((Box::new(meta), mem::take(&mut self.inner)));
    }

    /// End a layouting group started with [`start_group`].
    ///
    /// This returns the stored metadata and the collected nodes.
    ///
    /// [`start_group`]: #method.start_group
    pub fn end_group<T: 'static>(&mut self) -> (T, Vec<LayoutNode>) {
        let (any, outer) = self.groups.pop().expect("no pushed group");
        let group = *any.downcast::<T>().expect("bad group type");
        (group, mem::replace(&mut self.inner, outer))
    }

    /// Start a page run group based on the active page state.
    ///
    /// If `hard` is false, empty page runs will be omitted from the output.
    ///
    /// This also starts an inner paragraph.
    pub fn start_page_group(&mut self, hard: bool) {
        let size = self.state.page.size;
        let margins = self.state.page.margins();
        let dirs = self.state.dirs;
        let aligns = self.state.aligns;
        self.start_group((size, margins, dirs, aligns, hard));
        self.start_par_group();
    }

    /// End a page run group and push it to its parent group.
    ///
    /// This also ends an inner paragraph.
    pub fn end_page_group(&mut self) {
        self.end_par_group();
        let ((size, padding, dirs, aligns, hard), children) = self.end_group();
        let hard: bool = hard;
        if hard || !children.is_empty() {
            self.runs.push(Pages {
                size,
                child: LayoutNode::dynamic(Pad {
                    padding,
                    child: LayoutNode::dynamic(Stack {
                        dirs,
                        children,
                        aligns,
                        expand: Spec2::new(true, true),
                    }),
                }),
            })
        }
    }

    /// Start a paragraph group based on the active text state.
    pub fn start_par_group(&mut self) {
        let dirs = self.state.dirs;
        let line_spacing = self.state.text.line_spacing();
        let aligns = self.state.aligns;
        self.start_group((dirs, line_spacing, aligns));
    }

    /// End a paragraph group and push it to its parent group if its not empty.
    pub fn end_par_group(&mut self) {
        let ((dirs, line_spacing, aligns), children) = self.end_group();
        if !children.is_empty() {
            // FIXME: This is a hack and should be superseded by constraints
            //        having min and max size.
            let expand_cross = self.groups.len() <= 1;
            self.push(Par {
                dirs,
                line_spacing,
                children,
                aligns,
                expand: Gen2::new(false, expand_cross).switch(dirs),
            });
        }
    }

    /// Construct a text node from the given string based on the active text
    /// state.
    pub fn make_text_node(&self, text: String) -> Text {
        let mut variant = self.state.text.variant;

        if self.state.text.strong {
            variant.weight = variant.weight.thicken(300);
        }

        if self.state.text.emph {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        Text {
            text,
            dir: self.state.dirs.cross,
            size: self.state.text.font_size(),
            fallback: Rc::clone(&self.state.text.fallback),
            variant,
            aligns: self.state.aligns,
        }
    }
}

/// Evaluate an item.
///
/// _Note_: Evaluation is not necessarily pure, it may change the active state.
pub trait Eval {
    /// The output of evaluating the item.
    type Output;

    /// Evaluate the item to the output value.
    fn eval(&self, ctx: &mut EvalContext) -> Self::Output;
}

impl Eval for SynTree {
    type Output = ();

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        for node in self {
            node.v.eval(ctx);
        }
    }
}

impl Eval for SynNode {
    type Output = ();

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match self {
            SynNode::Space => {
                ctx.push(Spacing {
                    amount: ctx.state.text.word_spacing(),
                    softness: Softness::Soft,
                });
            }

            SynNode::Text(text) => {
                let node = ctx.make_text_node(text.clone());
                ctx.push(node);
            }

            SynNode::Linebreak => {
                ctx.end_par_group();
                ctx.start_par_group();
            }

            SynNode::Parbreak => {
                ctx.end_par_group();
                ctx.push(Spacing {
                    amount: ctx.state.text.par_spacing(),
                    softness: Softness::Soft,
                });
                ctx.start_par_group();
            }

            SynNode::Emph => {
                ctx.state.text.emph ^= true;
            }

            SynNode::Strong => {
                ctx.state.text.strong ^= true;
            }

            SynNode::Heading(heading) => {
                heading.eval(ctx);
            }

            SynNode::Raw(raw) => {
                raw.eval(ctx);
            }

            SynNode::Expr(expr) => {
                let value = expr.eval(ctx);
                value.eval(ctx);
            }
        }
    }
}

impl Eval for NodeHeading {
    type Output = ();

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let prev = ctx.state.clone();
        let upscale = 1.5 - 0.1 * self.level.v as f64;
        ctx.state.text.font_size.scale *= upscale;
        ctx.state.text.strong = true;

        self.contents.eval(ctx);

        ctx.state = prev;
    }
}

impl Eval for NodeRaw {
    type Output = ();

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let prev = Rc::clone(&ctx.state.text.fallback);
        let fallback = Rc::make_mut(&mut ctx.state.text.fallback);
        fallback.list.insert(0, "monospace".to_string());
        fallback.flatten();

        let mut children = vec![];
        for line in &self.lines {
            children.push(LayoutNode::Text(ctx.make_text_node(line.clone())));
        }

        ctx.push(Stack {
            dirs: ctx.state.dirs,
            children,
            aligns: ctx.state.aligns,
            expand: Spec2::new(false, false),
        });

        ctx.state.text.fallback = prev;
    }
}

impl Eval for Expr {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match self {
            Self::Lit(lit) => lit.eval(ctx),
            Self::Call(call) => call.eval(ctx),
            Self::Unary(unary) => unary.eval(ctx),
            Self::Binary(binary) => binary.eval(ctx),
        }
    }
}

impl Eval for Lit {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        match *self {
            Lit::Ident(ref v) => Value::Ident(v.clone()),
            Lit::Bool(v) => Value::Bool(v),
            Lit::Int(v) => Value::Int(v),
            Lit::Float(v) => Value::Float(v),
            Lit::Length(v) => Value::Length(v.as_raw()),
            Lit::Percent(v) => Value::Relative(v / 100.0),
            Lit::Color(v) => Value::Color(v),
            Lit::Str(ref v) => Value::Str(v.clone()),
            Lit::Dict(ref v) => Value::Dict(v.eval(ctx)),
            Lit::Content(ref v) => Value::Content(v.clone()),
        }
    }
}
impl Eval for LitDict {
    type Output = ValueDict;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let mut dict = ValueDict::new();

        for entry in &self.0 {
            let val = entry.expr.v.eval(ctx);
            let spanned = val.span_with(entry.expr.span);
            if let Some(key) = &entry.key {
                dict.insert(&key.v, SpannedEntry::new(key.span, spanned));
            } else {
                dict.push(SpannedEntry::value(spanned));
            }
        }

        dict
    }
}

impl Eval for ExprCall {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let name = &self.name.v;
        let span = self.name.span;
        let dict = self.args.v.eval(ctx);

        if let Some(func) = ctx.state.scope.get(name) {
            let args = Args(dict.span_with(self.args.span));
            ctx.f.decos.push(Deco::Resolved.span_with(span));
            (func.clone())(args, ctx)
        } else {
            if !name.is_empty() {
                ctx.diag(error!(span, "unknown function"));
                ctx.f.decos.push(Deco::Unresolved.span_with(span));
            }
            Value::Dict(dict)
        }
    }
}

impl Eval for ExprUnary {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        use Value::*;

        let value = self.expr.v.eval(ctx);
        if value == Error {
            return Error;
        }

        let span = self.op.span.join(self.expr.span);
        match self.op.v {
            UnOp::Neg => neg(ctx, span, value),
        }
    }
}

impl Eval for ExprBinary {
    type Output = Value;

    fn eval(&self, ctx: &mut EvalContext) -> Self::Output {
        let lhs = self.lhs.v.eval(ctx);
        let rhs = self.rhs.v.eval(ctx);

        if lhs == Value::Error || rhs == Value::Error {
            return Value::Error;
        }

        let span = self.lhs.span.join(self.rhs.span);
        match self.op.v {
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
            ctx.diag(error!(span, "cannot negate {}", v.ty()));
            Value::Error
        }
    }
}

/// Compute the sum of two values.
fn add(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use crate::geom::Linear as Lin;
    use Value::*;
    match (lhs, rhs) {
        // Numbers to themselves.
        (Int(a), Int(b)) => Int(a + b),
        (Int(a), Float(b)) => Float(a as f64 + b),
        (Float(a), Int(b)) => Float(a + b as f64),
        (Float(a), Float(b)) => Float(a + b),

        // Lengths, relatives and linears to themselves.
        (Length(a), Length(b)) => Length(a + b),
        (Length(a), Relative(b)) => Linear(Lin::abs(a) + Lin::rel(b)),
        (Length(a), Linear(b)) => Linear(Lin::abs(a) + b),

        (Relative(a), Length(b)) => Linear(Lin::rel(a) + Lin::abs(b)),
        (Relative(a), Relative(b)) => Relative(a + b),
        (Relative(a), Linear(b)) => Linear(Lin::rel(a) + b),

        (Linear(a), Length(b)) => Linear(a + Lin::abs(b)),
        (Linear(a), Relative(b)) => Linear(a + Lin::rel(b)),
        (Linear(a), Linear(b)) => Linear(a + b),

        // Complex data types to themselves.
        (Str(a), Str(b)) => Str(a + &b),
        (Dict(a), Dict(b)) => Dict(concat(a, b)),
        (Content(a), Content(b)) => Content(concat(a, b)),

        (a, b) => {
            ctx.diag(error!(span, "cannot add {} and {}", a.ty(), b.ty()));
            Value::Error
        }
    }
}

/// Compute the difference of two values.
fn sub(ctx: &mut EvalContext, span: Span, lhs: Value, rhs: Value) -> Value {
    use crate::geom::Linear as Lin;
    use Value::*;
    match (lhs, rhs) {
        // Numbers from themselves.
        (Int(a), Int(b)) => Int(a - b),
        (Int(a), Float(b)) => Float(a as f64 - b),
        (Float(a), Int(b)) => Float(a - b as f64),
        (Float(a), Float(b)) => Float(a - b),

        // Lengths, relatives and linears from themselves.
        (Length(a), Length(b)) => Length(a - b),
        (Length(a), Relative(b)) => Linear(Lin::abs(a) - Lin::rel(b)),
        (Length(a), Linear(b)) => Linear(Lin::abs(a) - b),
        (Relative(a), Length(b)) => Linear(Lin::rel(a) - Lin::abs(b)),
        (Relative(a), Relative(b)) => Relative(a - b),
        (Relative(a), Linear(b)) => Linear(Lin::rel(a) - b),
        (Linear(a), Length(b)) => Linear(a - Lin::abs(b)),
        (Linear(a), Relative(b)) => Linear(a - Lin::rel(b)),
        (Linear(a), Linear(b)) => Linear(a - b),

        (a, b) => {
            ctx.diag(error!(span, "cannot subtract {1} from {0}", a.ty(), b.ty()));
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
        (Int(a), Str(b)) => Str(b.repeat(a.max(0) as usize)),
        (Str(a), Int(b)) => Str(a.repeat(b.max(0) as usize)),

        (a, b) => {
            ctx.diag(error!(span, "cannot multiply {} with {}", a.ty(), b.ty()));
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
            ctx.diag(error!(span, "cannot divide {} by {}", a.ty(), b.ty()));
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

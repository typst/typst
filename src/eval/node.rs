use std::convert::TryFrom;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::mem;
use std::ops::{Add, AddAssign};

use super::Styles;
use crate::diag::StrResult;
use crate::geom::SpecAxis;
use crate::layout::{Layout, PackedNode, RootNode};
use crate::library::{
    FlowChild, FlowNode, PageNode, ParChild, ParNode, PlacedNode, SpacingKind,
    SpacingNode, TextNode,
};
use crate::util::EcoString;

/// A partial representation of a layout node.
///
/// A node is a composable intermediate representation that can be converted
/// into a proper layout node by lifting it to a [block-level](PackedNode) or
/// [root node](RootNode).
///
/// When you write `[Hi] + [you]` in Typst, this type's [`Add`] implementation
/// is invoked. There, multiple nodes are combined into a single
/// [`Sequence`](Self::Sequence) node.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Node {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak,
    /// A page break.
    Pagebreak,
    /// Plain text.
    Text(EcoString),
    /// Spacing.
    Spacing(SpecAxis, SpacingKind),
    /// An inline node.
    Inline(PackedNode),
    /// A block node.
    Block(PackedNode),
    /// A page node.
    Page(PageNode),
    /// Multiple nodes with attached styles.
    ///
    /// For example, the Typst template `[Hi *you!*]` would result in the
    /// sequence:
    /// ```ignore
    /// Sequence([
    ///   (Text("Hi"), {}),
    ///   (Space, {}),
    ///   (Text("you!"), { TextNode::STRONG: true }),
    /// ])
    /// ```
    /// A sequence may contain nested sequences (meaning this variant
    /// effectively allows nodes to form trees). All nested sequences can
    /// equivalently be represented as a single flat sequence, but allowing
    /// nesting doesn't hurt since we can just recurse into the nested sequences
    /// during packing. Also, in theory, this allows better complexity when
    /// adding (large) sequence nodes (just like for a text rope).
    Sequence(Vec<(Self, Styles)>),
}

impl Node {
    /// Create an empty node.
    pub fn new() -> Self {
        Self::Sequence(vec![])
    }

    /// Create an inline-level node.
    pub fn inline<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + 'static,
    {
        Self::Inline(node.pack())
    }

    /// Create a block-level node.
    pub fn block<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + 'static,
    {
        Self::Block(node.pack())
    }

    /// Style this node.
    pub fn styled(self, styles: Styles) -> Self {
        match self {
            Self::Inline(inline) => Self::Inline(inline.styled(styles)),
            Self::Block(block) => Self::Block(block.styled(styles)),
            Self::Page(page) => Self::Page(page.styled(styles)),
            other => Self::Sequence(vec![(other, styles)]),
        }
    }

    /// Style this node in monospace.
    pub fn monospaced(self) -> Self {
        self.styled(Styles::one(TextNode::MONOSPACE, true))
    }

    /// Lift to a type-erased block-level node.
    pub fn into_block(self) -> PackedNode {
        if let Node::Block(packed) = self {
            packed
        } else {
            let mut packer = Packer::new(false);
            packer.walk(self, Styles::new());
            packer.into_block()
        }
    }

    /// Lift to a root layout tree node.
    pub fn into_root(self) -> RootNode {
        let mut packer = Packer::new(true);
        packer.walk(self, Styles::new());
        packer.into_root()
    }

    /// Repeat this node `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        // TODO(style): Make more efficient.
        Ok(Self::Sequence(vec![(self.clone(), Styles::new()); count]))
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

impl Add for Node {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // TODO(style): Make more efficient.
        Self::Sequence(vec![(self, Styles::new()), (rhs, Styles::new())])
    }
}

impl AddAssign for Node {
    fn add_assign(&mut self, rhs: Self) {
        *self = mem::take(self) + rhs;
    }
}

impl Sum for Node {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::Sequence(iter.map(|n| (n, Styles::new())).collect())
    }
}

/// Packs a [`Node`] into a flow or root node.
struct Packer {
    /// Whether this packer produces a root node.
    top: bool,
    /// The accumulated page nodes.
    pages: Vec<PageNode>,
    /// The accumulated flow children.
    flow: Builder<FlowChild>,
    /// The accumulated paragraph children.
    par: Builder<ParChild>,
}

impl Packer {
    /// Start a new node-packing session.
    fn new(top: bool) -> Self {
        Self {
            top,
            pages: vec![],
            flow: Builder::default(),
            par: Builder::default(),
        }
    }

    /// Finish up and return the resulting flow.
    fn into_block(mut self) -> PackedNode {
        self.parbreak(None);
        FlowNode(self.flow.children).pack()
    }

    /// Finish up and return the resulting root node.
    fn into_root(mut self) -> RootNode {
        self.pagebreak();
        RootNode(self.pages)
    }

    /// Consider a node with the given styles.
    fn walk(&mut self, node: Node, styles: Styles) {
        match node {
            Node::Space => {
                // A text space is "soft", meaning that it can be eaten up by
                // adjacent line breaks or explicit spacings.
                self.par.last.soft(ParChild::text(' ', styles));
            }
            Node::Linebreak => {
                // A line break eats up surrounding text spaces.
                self.par.last.hard();
                self.push_inline(ParChild::text('\n', styles));
                self.par.last.hard();
            }
            Node::Parbreak => {
                // An explicit paragraph break is styled according to the active
                // styles (`Some(_)`) whereas paragraph breaks forced by
                // incompatibility take their styles from the preceding
                // paragraph.
                self.parbreak(Some(styles));
            }
            Node::Colbreak => {
                // Explicit column breaks end the current paragraph and then
                // discards the paragraph break.
                self.parbreak(None);
                self.make_flow_compatible(&styles);
                self.flow.children.push(FlowChild::Skip);
                self.flow.last.hard();
            }
            Node::Pagebreak => {
                // We must set the flow styles after the page break such that an
                // empty page created by two page breaks in a row has styles at
                // all.
                self.pagebreak();
                self.flow.styles = styles;
            }
            Node::Text(text) => {
                self.push_inline(ParChild::text(text, styles));
            }
            Node::Spacing(SpecAxis::Horizontal, kind) => {
                // Just like a line break, explicit horizontal spacing eats up
                // surrounding text spaces.
                self.par.last.hard();
                self.push_inline(ParChild::Spacing(SpacingNode { kind, styles }));
                self.par.last.hard();
            }
            Node::Spacing(SpecAxis::Vertical, kind) => {
                // Explicit vertical spacing ends the current paragraph and then
                // discards the paragraph break.
                self.parbreak(None);
                self.make_flow_compatible(&styles);
                self.flow
                    .children
                    .push(FlowChild::Spacing(SpacingNode { kind, styles }));
                self.flow.last.hard();
            }
            Node::Inline(inline) => {
                self.push_inline(ParChild::Node(inline.styled(styles)));
            }
            Node::Block(block) => {
                self.push_block(block.styled(styles));
            }
            Node::Page(page) => {
                if self.top {
                    self.pagebreak();
                    self.pages.push(page.styled(styles));
                } else {
                    let flow = page.child.styled(page.styles);
                    self.push_block(flow.styled(styles));
                }
            }
            Node::Sequence(list) => {
                // For a list of nodes, we apply the list's styles to each node
                // individually.
                for (node, mut inner) in list {
                    inner.apply(&styles);
                    self.walk(node, inner);
                }
            }
        }
    }

    /// Insert an inline-level element into the current paragraph.
    fn push_inline(&mut self, child: ParChild) {
        if let Some(child) = self.par.last.any() {
            self.push_coalescing(child);
        }

        // The node must be both compatible with the current page and the
        // current paragraph.
        self.make_flow_compatible(child.styles());
        self.make_par_compatible(child.styles());
        self.push_coalescing(child);
        self.par.last.any();
    }

    /// Push a paragraph child, coalescing text nodes with compatible styles.
    fn push_coalescing(&mut self, child: ParChild) {
        if let ParChild::Text(right) = &child {
            if let Some(ParChild::Text(left)) = self.par.children.last_mut() {
                if left.styles.compatible(&right.styles, TextNode::has_property) {
                    left.text.push_str(&right.text);
                    return;
                }
            }
        }

        self.par.children.push(child);
    }

    /// Insert a block-level element into the current flow.
    fn push_block(&mut self, node: PackedNode) {
        let placed = node.is::<PlacedNode>();

        self.parbreak(None);
        self.make_flow_compatible(&node.styles);
        self.flow.children.extend(self.flow.last.any());
        self.flow.children.push(FlowChild::Node(node));
        self.parbreak(None);

        // Prevent paragraph spacing between the placed node and the paragraph
        // below it.
        if placed {
            self.flow.last.hard();
        }
    }

    /// Advance to the next paragraph.
    fn parbreak(&mut self, break_styles: Option<Styles>) {
        // Erase any styles that will be inherited anyway.
        let Builder { mut children, styles, .. } = mem::take(&mut self.par);
        for child in &mut children {
            child.styles_mut().erase(&styles);
        }

        // For explicit paragraph breaks, `break_styles` is already `Some(_)`.
        // For page breaks due to incompatibility, we fall back to the styles
        // of the preceding paragraph.
        let break_styles = break_styles.unwrap_or_else(|| styles.clone());

        // We don't want empty paragraphs.
        if !children.is_empty() {
            // The paragraph's children are all compatible with the page, so the
            // paragraph is too, meaning we don't need to check or intersect
            // anything here.
            let par = ParNode(children).pack().styled(styles);
            self.flow.children.extend(self.flow.last.any());
            self.flow.children.push(FlowChild::Node(par));
        }

        // Insert paragraph spacing.
        self.flow.last.soft(FlowChild::Break(break_styles));
    }

    /// Advance to the next page.
    fn pagebreak(&mut self) {
        if self.top {
            self.parbreak(None);

            // Take the flow and erase any styles that will be inherited anyway.
            let Builder { mut children, styles, .. } = mem::take(&mut self.flow);
            for child in &mut children {
                child.styles_mut().map(|s| s.erase(&styles));
            }

            let flow = FlowNode(children).pack();
            let page = PageNode { child: flow, styles };
            self.pages.push(page);
        }
    }

    /// Break to a new paragraph if the `styles` contain paragraph styles that
    /// are incompatible with the current paragraph.
    fn make_par_compatible(&mut self, styles: &Styles) {
        if self.par.children.is_empty() {
            self.par.styles = styles.clone();
            return;
        }

        if !self.par.styles.compatible(&styles, ParNode::has_property) {
            self.parbreak(None);
            self.par.styles = styles.clone();
            return;
        }

        self.par.styles.intersect(&styles);
    }

    /// Break to a new page if the `styles` contain page styles that are
    /// incompatible with the current flow.
    fn make_flow_compatible(&mut self, styles: &Styles) {
        if self.flow.children.is_empty() && self.par.children.is_empty() {
            self.flow.styles = styles.clone();
            return;
        }

        if self.top && !self.flow.styles.compatible(&styles, PageNode::has_property) {
            self.pagebreak();
            self.flow.styles = styles.clone();
            return;
        }

        self.flow.styles.intersect(styles);
    }
}

/// Container for building a flow or paragraph.
struct Builder<T> {
    /// The intersection of the style properties of all `children`.
    styles: Styles,
    /// The accumulated flow or paragraph children.
    children: Vec<T>,
    /// The kind of thing that was last added.
    last: Last<T>,
}

impl<T> Default for Builder<T> {
    fn default() -> Self {
        Self {
            styles: Styles::new(),
            children: vec![],
            last: Last::None,
        }
    }
}

/// The kind of node that was last added to a flow or paragraph. A small finite
/// state machine used to coalesce spaces.
///
/// Soft nodes can only exist when surrounded by `Any` nodes. Not at the
/// start, end or next to hard nodes. This way, spaces at start and end of
/// paragraphs and next to `#h(..)` goes away.
enum Last<N> {
    /// Start state, nothing there.
    None,
    /// Text or a block node or something.
    Any,
    /// Hard nodes: Linebreaks and explicit spacing.
    Hard,
    /// Soft nodes: Word spaces and paragraph breaks. These are saved here
    /// temporarily and then applied once an `Any` node appears.
    Soft(N),
}

impl<N> Last<N> {
    /// Transition into the `Any` state and return a soft node to really add
    /// now if currently in `Soft` state.
    fn any(&mut self) -> Option<N> {
        match mem::replace(self, Self::Any) {
            Self::Soft(soft) => Some(soft),
            _ => None,
        }
    }

    /// Transition into the `Soft` state, but only if in `Any`. Otherwise, the
    /// soft node is discarded.
    fn soft(&mut self, soft: N) {
        if let Self::Any = self {
            *self = Self::Soft(soft);
        }
    }

    /// Transition into the `Hard` state, discarding a possibly existing soft
    /// node and preventing further soft nodes from being added.
    fn hard(&mut self) {
        *self = Self::Hard;
    }
}

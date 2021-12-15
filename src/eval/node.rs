use std::convert::TryFrom;
use std::fmt::Debug;
use std::hash::Hash;
use std::mem;
use std::ops::{Add, AddAssign};

use super::Styles;
use crate::diag::StrResult;
use crate::geom::SpecAxis;
use crate::layout::{Layout, PackedNode};
use crate::library::{
    DocumentNode, FlowChild, FlowNode, PageNode, ParChild, ParNode, PlacedNode,
    SpacingKind, SpacingNode, TextNode,
};
use crate::util::EcoString;

/// A partial representation of a layout node.
///
/// A node is a composable intermediate representation that can be converted
/// into a proper layout node by lifting it to a block-level or document node.
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Node {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
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
    /// A sequence of nodes (which may themselves contain sequences).
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
            let mut packer = NodePacker::new(true);
            packer.walk(self, Styles::new());
            packer.into_block()
        }
    }

    /// Lift to a document node, the root of the layout tree.
    pub fn into_document(self) -> DocumentNode {
        let mut packer = NodePacker::new(false);
        packer.walk(self, Styles::new());
        packer.into_document()
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        // TODO(set): Make more efficient.
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
        // TODO(set): Make more efficient.
        Self::Sequence(vec![(self, Styles::new()), (rhs, Styles::new())])
    }
}

impl AddAssign for Node {
    fn add_assign(&mut self, rhs: Self) {
        *self = mem::take(self) + rhs;
    }
}

/// Packs a [`Node`] into a flow or whole document.
struct NodePacker {
    /// Whether packing should produce a block-level node.
    block: bool,
    /// The accumulated page nodes.
    pages: Vec<PageNode>,
    /// The accumulated flow children.
    flow: Vec<FlowChild>,
    /// The common style properties of all items on the current flow.
    flow_styles: Styles,
    /// The kind of thing that was last added to the current flow.
    flow_last: Last<FlowChild>,
    /// The accumulated paragraph children.
    par: Vec<ParChild>,
    /// The common style properties of all items in the current paragraph.
    par_styles: Styles,
    /// The kind of thing that was last added to the current paragraph.
    par_last: Last<ParChild>,
}

impl NodePacker {
    /// Start a new node-packing session.
    fn new(block: bool) -> Self {
        Self {
            block,
            pages: vec![],
            flow: vec![],
            flow_styles: Styles::new(),
            flow_last: Last::None,
            par: vec![],
            par_styles: Styles::new(),
            par_last: Last::None,
        }
    }

    /// Finish up and return the resulting flow.
    fn into_block(mut self) -> PackedNode {
        self.finish_par();
        FlowNode(self.flow).pack()
    }

    /// Finish up and return the resulting document.
    fn into_document(mut self) -> DocumentNode {
        self.pagebreak(true);
        DocumentNode(self.pages)
    }

    /// Consider a node with the given styles.
    fn walk(&mut self, node: Node, styles: Styles) {
        match node {
            Node::Space => {
                if self.is_flow_compatible(&styles) && self.is_par_compatible(&styles) {
                    self.par_last.soft(ParChild::text(' ', styles));
                }
            }
            Node::Linebreak => {
                self.par_last.hard();
                self.push_inline(ParChild::text('\n', styles));
                self.par_last.hard();
            }
            Node::Parbreak => {
                self.parbreak(Some(styles));
            }
            Node::Pagebreak => {
                self.pagebreak(true);
                self.flow_styles = styles;
            }
            Node::Text(text) => {
                self.push_inline(ParChild::text(text, styles));
            }
            Node::Spacing(SpecAxis::Horizontal, kind) => {
                self.par_last.hard();
                self.push_inline(ParChild::Spacing(SpacingNode { kind, styles }));
                self.par_last.hard();
            }
            Node::Spacing(SpecAxis::Vertical, kind) => {
                self.finish_par();
                self.flow.push(FlowChild::Spacing(SpacingNode { kind, styles }));
                self.flow_last.hard();
            }
            Node::Inline(inline) => {
                self.push_inline(ParChild::Node(inline.styled(styles)));
            }
            Node::Block(block) => {
                self.push_block(block.styled(styles));
            }
            Node::Sequence(list) => {
                for (node, mut inner) in list {
                    inner.apply(&styles);
                    self.walk(node, inner);
                }
            }
        }
    }

    /// Insert an inline-level element into the current paragraph.
    fn push_inline(&mut self, child: ParChild) {
        if let Some(child) = self.par_last.any() {
            self.push_inline_impl(child);
        }

        // The node must be both compatible with the current page and the
        // current paragraph.
        self.make_flow_compatible(child.styles());
        self.make_par_compatible(child.styles());
        self.push_inline_impl(child);
        self.par_last = Last::Any;
    }

    /// Push a paragraph child, coalescing text nodes with compatible styles.
    fn push_inline_impl(&mut self, child: ParChild) {
        if let ParChild::Text(right) = &child {
            if let Some(ParChild::Text(left)) = self.par.last_mut() {
                if left.styles.compatible(&right.styles, TextNode::has_property) {
                    left.text.push_str(&right.text);
                    return;
                }
            }
        }

        self.par.push(child);
    }

    /// Insert a block-level element into the current flow.
    fn push_block(&mut self, node: PackedNode) {
        let mut is_placed = false;
        if let Some(placed) = node.downcast::<PlacedNode>() {
            is_placed = true;

            // This prevents paragraph spacing after the placed node if it
            // is completely out-of-flow.
            if placed.out_of_flow() {
                self.flow_last = Last::None;
            }
        }

        self.parbreak(None);
        self.make_flow_compatible(&node.styles);

        if let Some(child) = self.flow_last.any() {
            self.flow.push(child);
        }

        self.flow.push(FlowChild::Node(node));
        self.parbreak(None);

        // This prevents paragraph spacing between the placed node and
        // the paragraph below it.
        if is_placed {
            self.flow_last = Last::None;
        }
    }

    /// Advance to the next paragraph.
    fn parbreak(&mut self, break_styles: Option<Styles>) {
        self.finish_par();

        // Insert paragraph spacing.
        self.flow_last
            .soft(FlowChild::Parbreak(break_styles.unwrap_or_default()));
    }

    fn finish_par(&mut self) {
        let mut children = mem::take(&mut self.par);
        let styles = mem::take(&mut self.par_styles);
        self.par_last = Last::None;

        // No empty paragraphs.
        if !children.is_empty() {
            // Erase any styles that will be inherited anyway.
            for child in &mut children {
                child.styles_mut().erase(&styles);
            }

            if let Some(child) = self.flow_last.any() {
                self.flow.push(child);
            }

            // The paragraph's children are all compatible with the page, so the
            // paragraph is too, meaning we don't need to check or intersect
            // anything here.
            let node = ParNode(children).pack().styled(styles);
            self.flow.push(FlowChild::Node(node));
        }
    }

    /// Advance to the next page.
    fn pagebreak(&mut self, keep: bool) {
        if self.block {
            return;
        }

        self.finish_par();

        let styles = mem::take(&mut self.flow_styles);
        let mut children = mem::take(&mut self.flow);
        self.flow_last = Last::None;

        if keep || !children.is_empty() {
            // Erase any styles that will be inherited anyway.
            for child in &mut children {
                child.styles_mut().erase(&styles);
            }

            let node = PageNode { node: FlowNode(children).pack(), styles };
            self.pages.push(node);
        }
    }

    /// Break to a new paragraph if the `styles` contain paragraph styles that
    /// are incompatible with the current paragraph.
    fn make_par_compatible(&mut self, styles: &Styles) {
        if self.par.is_empty() {
            self.par_styles = styles.clone();
            return;
        }

        if !self.is_par_compatible(styles) {
            self.parbreak(None);
            self.par_styles = styles.clone();
            return;
        }

        self.par_styles.intersect(&styles);
    }

    /// Break to a new page if the `styles` contain page styles that are
    /// incompatible with the current flow.
    fn make_flow_compatible(&mut self, styles: &Styles) {
        if self.flow.is_empty() && self.par.is_empty() {
            self.flow_styles = styles.clone();
            return;
        }

        if !self.is_flow_compatible(styles) {
            self.pagebreak(false);
            self.flow_styles = styles.clone();
            return;
        }

        self.flow_styles.intersect(styles);
    }

    /// Whether the given styles are compatible with the current page.
    fn is_par_compatible(&self, styles: &Styles) -> bool {
        self.par_styles.compatible(&styles, ParNode::has_property)
    }

    /// Whether the given styles are compatible with the current flow.
    fn is_flow_compatible(&self, styles: &Styles) -> bool {
        self.block || self.flow_styles.compatible(&styles, PageNode::has_property)
    }
}

/// Finite state machine for spacing coalescing.
enum Last<N> {
    None,
    Any,
    Soft(N),
}

impl<N> Last<N> {
    fn any(&mut self) -> Option<N> {
        match mem::replace(self, Self::Any) {
            Self::Soft(soft) => Some(soft),
            _ => None,
        }
    }

    fn soft(&mut self, soft: N) {
        if let Self::Any = self {
            *self = Self::Soft(soft);
        }
    }

    fn hard(&mut self) {
        *self = Self::None;
    }
}

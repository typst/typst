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
    Decoration, DocumentNode, FlowChild, FlowNode, PageNode, ParChild, ParNode, Spacing,
    TextNode,
};
use crate::util::EcoString;

/// A partial representation of a layout node.
///
/// A node is a composable intermediate representation that can be converted
/// into a proper layout node by lifting it to a block-level or document node.
// TODO(set): Fix Debug impl leaking into user-facing repr.
#[derive(Debug, Clone)]
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
    Spacing(SpecAxis, Spacing),
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

    /// Decorate this node.
    pub fn decorated(self, _: Decoration) -> Self {
        // TODO(set): Actually decorate.
        self
    }

    /// Lift to a type-erased block-level node.
    pub fn into_block(self) -> PackedNode {
        if let Node::Block(packed) = self {
            packed
        } else {
            let mut packer = NodePacker::new();
            packer.walk(self, Styles::new());
            packer.into_block()
        }
    }

    /// Lift to a document node, the root of the layout tree.
    pub fn into_document(self) -> DocumentNode {
        let mut packer = NodePacker::new();
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

impl PartialEq for Node {
    fn eq(&self, _: &Self) -> bool {
        // TODO(set): Figure out what to do here.
        false
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
    /// The accumulated page nodes.
    document: Vec<PageNode>,
    /// The common style properties of all items on the current page.
    page_styles: Styles,
    /// The accumulated flow children.
    flow: Vec<FlowChild>,
    /// The accumulated paragraph children.
    par: Vec<ParChild>,
    /// The common style properties of all items in the current paragraph.
    par_styles: Styles,
    /// The kind of thing that was last added to the current paragraph.
    last: Last,
}

/// The type of the last thing that was pushed into the paragraph.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Last {
    None,
    Spacing,
    Newline,
    Space,
    Other,
}

impl NodePacker {
    /// Start a new node-packing session.
    fn new() -> Self {
        Self {
            document: vec![],
            page_styles: Styles::new(),
            flow: vec![],
            par: vec![],
            par_styles: Styles::new(),
            last: Last::None,
        }
    }

    /// Finish up and return the resulting flow.
    fn into_block(mut self) -> PackedNode {
        self.parbreak();
        FlowNode(self.flow).pack()
    }

    /// Finish up and return the resulting document.
    fn into_document(mut self) -> DocumentNode {
        self.pagebreak(true);
        DocumentNode(self.document)
    }

    /// Consider a node with the given styles.
    fn walk(&mut self, node: Node, styles: Styles) {
        match node {
            Node::Space => {
                // Only insert a space if the previous thing was actual content.
                if self.last == Last::Other {
                    self.push_text(' '.into(), styles);
                    self.last = Last::Space;
                }
            }
            Node::Linebreak => {
                self.trim();
                self.push_text('\n'.into(), styles);
                self.last = Last::Newline;
            }
            Node::Parbreak => {
                self.parbreak();
            }
            Node::Pagebreak => {
                self.pagebreak(true);
                self.page_styles = styles;
            }
            Node::Text(text) => {
                self.push_text(text, styles);
            }
            Node::Spacing(SpecAxis::Horizontal, amount) => {
                self.push_inline(ParChild::Spacing(amount), styles);
                self.last = Last::Spacing;
            }
            Node::Spacing(SpecAxis::Vertical, amount) => {
                self.push_block(FlowChild::Spacing(amount), styles);
            }
            Node::Inline(inline) => {
                self.push_inline(ParChild::Node(inline), styles);
            }
            Node::Block(block) => {
                self.push_block(FlowChild::Node(block), styles);
            }
            Node::Sequence(list) => {
                for (node, mut inner) in list {
                    inner.apply(&styles);
                    self.walk(node, inner);
                }
            }
        }
    }

    /// Remove a trailing space.
    fn trim(&mut self) {
        if self.last == Last::Space {
            self.par.pop();
            self.last = Last::Other;
        }
    }

    /// Advance to the next paragraph.
    fn parbreak(&mut self) {
        self.trim();

        let children = mem::take(&mut self.par);
        let styles = mem::take(&mut self.par_styles);
        if !children.is_empty() {
            // The paragraph's children are all compatible with the page, so the
            // paragraph is too, meaning we don't need to check or intersect
            // anything here.
            let node = ParNode(children).pack().styled(styles);
            self.flow.push(FlowChild::Node(node));
        }

        self.last = Last::None;
    }

    /// Advance to the next page.
    fn pagebreak(&mut self, keep: bool) {
        self.parbreak();
        let children = mem::take(&mut self.flow);
        let styles = mem::take(&mut self.page_styles);
        if keep || !children.is_empty() {
            let node = PageNode { node: FlowNode(children).pack(), styles };
            self.document.push(node);
        }
    }

    /// Insert text into the current paragraph.
    fn push_text(&mut self, text: EcoString, styles: Styles) {
        // TODO(set): Join compatible text nodes. Take care with space
        // coalescing.
        let node = TextNode { text, styles: Styles::new() };
        self.push_inline(ParChild::Text(node), styles);
    }

    /// Insert an inline-level element into the current paragraph.
    fn push_inline(&mut self, mut child: ParChild, styles: Styles) {
        match &mut child {
            ParChild::Spacing(_) => {}
            ParChild::Text(node) => node.styles.apply(&styles),
            ParChild::Node(node) => node.styles.apply(&styles),
            ParChild::Decorate(_) => {}
            ParChild::Undecorate => {}
        }

        // The node must be both compatible with the current page and the
        // current paragraph.
        self.make_page_compatible(&styles);
        self.make_par_compatible(&styles);
        self.par.push(child);
        self.last = Last::Other;
    }

    /// Insert a block-level element into the current flow.
    fn push_block(&mut self, mut child: FlowChild, styles: Styles) {
        self.parbreak();

        match &mut child {
            FlowChild::Spacing(_) => {}
            FlowChild::Node(node) => node.styles.apply(&styles),
        }

        // The node must be compatible with the current page.
        self.make_page_compatible(&styles);
        self.flow.push(child);
    }

    /// Break to a new paragraph if the `styles` contain paragraph styles that
    /// are incompatible with the current paragraph.
    fn make_par_compatible(&mut self, styles: &Styles) {
        if self.par.is_empty() {
            self.par_styles = styles.clone();
            return;
        }

        if !self.par_styles.compatible(&styles, ParNode::has_property) {
            self.parbreak();
            self.par_styles = styles.clone();
            return;
        }

        self.par_styles.intersect(&styles);
    }

    /// Break to a new page if the `styles` contain page styles that are
    /// incompatible with the current page.
    fn make_page_compatible(&mut self, styles: &Styles) {
        if self.flow.is_empty() && self.par.is_empty() {
            self.page_styles = styles.clone();
            return;
        }

        if !self.page_styles.compatible(&styles, PageNode::has_property) {
            self.pagebreak(false);
            self.page_styles = styles.clone();
            return;
        }

        self.page_styles.intersect(styles);
    }
}

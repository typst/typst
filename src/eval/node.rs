use std::convert::TryFrom;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::mem;
use std::ops::{Add, AddAssign};

use crate::diag::StrResult;
use crate::geom::SpecAxis;
use crate::layout::{Layout, PackedNode};
use crate::library::{
    Decoration, DocumentNode, FlowChild, FlowNode, PageNode, ParChild, ParNode, Spacing,
};
use crate::util::EcoString;

/// A partial representation of a layout node.
///
/// A node is a composable intermediate representation that can be converted
/// into a proper layout node by lifting it to the block or page level.
#[derive(Clone)]
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
    Seq(Vec<Self>),
}

impl Node {
    /// Create an empty node.
    pub fn new() -> Self {
        Self::Seq(vec![])
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

    /// Decoration this node.
    pub fn decorate(self, _: Decoration) -> Self {
        // TODO(set): Actually decorate.
        self
    }

    /// Lift to a type-erased block-level node.
    pub fn into_block(self) -> PackedNode {
        if let Node::Block(packed) = self {
            packed
        } else {
            let mut packer = NodePacker::new();
            packer.walk(self);
            packer.into_block()
        }
    }

    /// Lift to a document node, the root of the layout tree.
    pub fn into_document(self) -> DocumentNode {
        let mut packer = NodePacker::new();
        packer.walk(self);
        packer.into_document()
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        // TODO(set): Make more efficient.
        Ok(Self::Seq(vec![self.clone(); count]))
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("<node>")
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
        Self::Seq(vec![self, rhs])
    }
}

impl AddAssign for Node {
    fn add_assign(&mut self, rhs: Self) {
        *self = mem::take(self) + rhs;
    }
}

/// Packs a `Node` into a flow or whole document.
struct NodePacker {
    document: Vec<PageNode>,
    flow: Vec<FlowChild>,
    par: Vec<ParChild>,
}

impl NodePacker {
    fn new() -> Self {
        Self {
            document: vec![],
            flow: vec![],
            par: vec![],
        }
    }

    fn into_block(mut self) -> PackedNode {
        self.parbreak();
        FlowNode(self.flow).pack()
    }

    fn into_document(mut self) -> DocumentNode {
        self.parbreak();
        self.pagebreak();
        DocumentNode(self.document)
    }

    fn walk(&mut self, node: Node) {
        match node {
            Node::Space => {
                self.push_inline(ParChild::Text(' '.into()));
            }
            Node::Linebreak => {
                self.push_inline(ParChild::Text('\n'.into()));
            }
            Node::Parbreak => {
                self.parbreak();
            }
            Node::Pagebreak => {
                self.pagebreak();
            }
            Node::Text(text) => {
                self.push_inline(ParChild::Text(text));
            }
            Node::Spacing(axis, amount) => match axis {
                SpecAxis::Horizontal => self.push_inline(ParChild::Spacing(amount)),
                SpecAxis::Vertical => self.push_block(FlowChild::Spacing(amount)),
            },
            Node::Inline(inline) => {
                self.push_inline(ParChild::Node(inline));
            }
            Node::Block(block) => {
                self.push_block(FlowChild::Node(block));
            }
            Node::Seq(list) => {
                for node in list {
                    self.walk(node);
                }
            }
        }
    }

    fn parbreak(&mut self) {
        let children = mem::take(&mut self.par);
        if !children.is_empty() {
            self.flow.push(FlowChild::Node(ParNode(children).pack()));
        }
    }

    fn pagebreak(&mut self) {
        let children = mem::take(&mut self.flow);
        self.document.push(PageNode(FlowNode(children).pack()));
    }

    fn push_inline(&mut self, child: ParChild) {
        self.par.push(child);
    }

    fn push_block(&mut self, child: FlowChild) {
        self.parbreak();
        self.flow.push(child);
    }
}

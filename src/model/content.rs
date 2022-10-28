use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::iter::Sum;
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use comemo::Tracked;

use super::{
    Builder, Dict, Key, Layout, LayoutNode, Property, Regions, Scratch, Selector, Show,
    ShowNode, StyleChain, StyleEntry, StyleMap,
};
use crate::diag::{SourceResult, StrResult};
use crate::frame::{Frame, Role};
use crate::geom::Abs;
use crate::library::layout::{PageNode, Spacing};
use crate::library::structure::ListItem;
use crate::util::EcoString;
use crate::World;

/// Composable representation of styled content.
///
/// This results from:
/// - anything written between square brackets in Typst
/// - any node constructor
///
/// Content is represented as a tree of nodes. There are two nodes of special
/// interest:
///
/// 1. A `Styled` node attaches a style map to other content. For example, a
///    single bold word could be represented as a `Styled(Text("Hello"),
///    [TextNode::STRONG: true])` node.
///
/// 2. A `Sequence` node content combines other arbitrary content and is the
///    representation of a "flow" of other nodes. So, when you write `[Hi] +
///    [you]` in Typst, this type's [`Add`] implementation is invoked and the
///    two [`Text`](Self::Text) nodes are combined into a single
///    [`Sequence`](Self::Sequence) node. A sequence may contain nested
///    sequences.
#[derive(PartialEq, Clone, Hash)]
pub enum Content {
    /// Empty content.
    Empty,
    /// A word space.
    Space,
    /// A forced line break.
    Linebreak { justify: bool },
    /// Horizontal spacing.
    Horizontal { amount: Spacing, weak: bool },
    /// Plain text.
    Text(EcoString),
    /// A smart quote.
    Quote { double: bool },
    /// An inline-level node.
    Inline(LayoutNode),
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak { weak: bool },
    /// Vertical spacing.
    Vertical {
        amount: Spacing,
        weak: bool,
        generated: bool,
    },
    /// A block-level node.
    Block(LayoutNode),
    /// A list / enum item.
    Item(ListItem),
    /// A page break.
    Pagebreak { weak: bool },
    /// A page node.
    Page(PageNode),
    /// A node that can be realized with styles, optionally with attached
    /// properties.
    Show(ShowNode, Option<Dict>),
    /// Content with attached styles.
    Styled(Arc<(Self, StyleMap)>),
    /// A sequence of multiple nodes.
    Sequence(Arc<Vec<Self>>),
}

impl Content {
    /// Create empty content.
    pub fn new() -> Self {
        Self::Empty
    }

    /// Create content from an inline-level node.
    pub fn inline<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Inline(node.pack())
    }

    /// Create content from a block-level node.
    pub fn block<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Block(node.pack())
    }

    /// Create content from a showable node.
    pub fn show<T>(node: T) -> Self
    where
        T: Show + Debug + Hash + Sync + Send + 'static,
    {
        Self::Show(node.pack(), None)
    }

    /// Create a new sequence node from multiples nodes.
    pub fn sequence(seq: Vec<Self>) -> Self {
        match seq.as_slice() {
            [] => Self::Empty,
            [_] => seq.into_iter().next().unwrap(),
            _ => Self::Sequence(Arc::new(seq)),
        }
    }

    /// Repeat this content `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this content {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }

    /// Style this content with a single style property.
    pub fn styled<'k, K: Key<'k>>(self, key: K, value: K::Value) -> Self {
        self.styled_with_entry(StyleEntry::Property(Property::new(key, value)))
    }

    /// Style this content with a style entry.
    pub fn styled_with_entry(mut self, entry: StyleEntry) -> Self {
        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                map.apply(entry);
                return self;
            }
        }

        Self::Styled(Arc::new((self, entry.into())))
    }

    /// Style this content with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            return self;
        }

        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                map.apply_map(&styles);
                return self;
            }
        }

        Self::Styled(Arc::new((self, styles)))
    }

    /// Assign a semantic role to this content.
    pub fn role(self, role: Role) -> Self {
        self.styled_with_entry(StyleEntry::Role(role))
    }

    /// Reenable the show rule identified by the selector.
    pub fn unguard(&self, sel: Selector) -> Self {
        self.clone().styled_with_entry(StyleEntry::Unguard(sel))
    }

    /// Add weak vertical spacing above and below the node.
    pub fn spaced(self, above: Option<Abs>, below: Option<Abs>) -> Self {
        if above.is_none() && below.is_none() {
            return self;
        }

        let mut seq = vec![];
        if let Some(above) = above {
            seq.push(Content::Vertical {
                amount: above.into(),
                weak: true,
                generated: true,
            });
        }

        seq.push(self);
        if let Some(below) = below {
            seq.push(Content::Vertical {
                amount: below.into(),
                weak: true,
                generated: true,
            });
        }

        Self::sequence(seq)
    }
}

impl Layout for Content {
    fn layout(
        &self,
        world: Tracked<dyn World>,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let scratch = Scratch::default();
        let mut builder = Builder::new(world, &scratch, false);
        builder.accept(self, styles)?;
        let (flow, shared) = builder.into_flow(styles)?;
        flow.layout(world, regions, shared)
    }

    fn pack(self) -> LayoutNode {
        match self {
            Content::Block(node) => node,
            other => LayoutNode::new(other),
        }
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Content {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Empty => f.pad("Empty"),
            Self::Space => f.pad("Space"),
            Self::Linebreak { justify } => write!(f, "Linebreak({justify})"),
            Self::Horizontal { amount, weak } => {
                write!(f, "Horizontal({amount:?}, {weak})")
            }
            Self::Text(text) => write!(f, "Text({text:?})"),
            Self::Quote { double } => write!(f, "Quote({double})"),
            Self::Inline(node) => node.fmt(f),
            Self::Parbreak => f.pad("Parbreak"),
            Self::Colbreak { weak } => write!(f, "Colbreak({weak})"),
            Self::Vertical { amount, weak, generated } => {
                write!(f, "Vertical({amount:?}, {weak}, {generated})")
            }
            Self::Block(node) => node.fmt(f),
            Self::Item(item) => item.fmt(f),
            Self::Pagebreak { weak } => write!(f, "Pagebreak({weak})"),
            Self::Page(page) => page.fmt(f),
            Self::Show(node, _) => node.fmt(f),
            Self::Styled(styled) => {
                let (sub, map) = styled.as_ref();
                map.fmt(f)?;
                sub.fmt(f)
            }
            Self::Sequence(seq) => f.debug_list().entries(seq.iter()).finish(),
        }
    }
}

impl Add for Content {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Sequence(match (self, rhs) {
            (Self::Empty, rhs) => return rhs,
            (lhs, Self::Empty) => return lhs,
            (Self::Sequence(mut lhs), Self::Sequence(rhs)) => {
                let mutable = Arc::make_mut(&mut lhs);
                match Arc::try_unwrap(rhs) {
                    Ok(vec) => mutable.extend(vec),
                    Err(rc) => mutable.extend(rc.iter().cloned()),
                }
                lhs
            }
            (Self::Sequence(mut lhs), rhs) => {
                Arc::make_mut(&mut lhs).push(rhs);
                lhs
            }
            (lhs, Self::Sequence(mut rhs)) => {
                Arc::make_mut(&mut rhs).insert(0, lhs);
                rhs
            }
            (lhs, rhs) => Arc::new(vec![lhs, rhs]),
        })
    }
}

impl AddAssign for Content {
    fn add_assign(&mut self, rhs: Self) {
        *self = std::mem::take(self) + rhs;
    }
}

impl Sum for Content {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::sequence(iter.collect())
    }
}

use std::fmt::{self, Debug, Formatter};

use crate::foundations::{Content, cast, elem};
use crate::layout::{Dir, Spacing};

/// Arranges content and spacing horizontally or vertically.
///
/// The stack places a list of items along an axis, with optional spacing
/// between each item.
///
/// = Example <example>
/// ```example
/// #stack(
///   dir: ttb,
///   rect(width: 40pt),
///   rect(width: 120pt),
///   rect(width: 90pt),
/// )
/// ```
///
/// = Accessibility <accessibility>
/// Stacks do not carry any special semantics. The contents of the stack are
/// read by Assistive Technology (AT) in the order in which they have been
/// passed to this function.
#[elem(since = "forever")]
pub struct StackElem {
    /// The direction along which the items are stacked.
    ///
    /// ```example
    /// #stack(dir: ltr, "A", "B", "C")
    ///
    /// #stack(dir: btt, "A", "B", "C")
    /// ```
    #[default(Dir::TTB)]
    pub dir: Dir,

    /// Spacing to insert between items where no explicit spacing was provided.
    ///
    /// ```example
    /// #stack(
    ///   dir: ltr,
    ///   spacing: 1fr,
    ///   [A], [B], [C],
    /// )
    /// ```
    pub spacing: Option<Spacing>,

    /// The children to stack along the axis.
    ///
    /// Each child can be some content or explicit spacing that replaces the
    /// spacing from the `spacing` parameter.
    ///
    /// Note that explicit spacing can also be specified as a @h or @v element
    /// depending on the stack direction.
    ///
    /// ```example
    /// #stack(
    ///   dir: ltr,
    ///   spacing: 10pt,
    ///   [A], [B], 0pt, [C],
    /// )
    /// ```
    #[variadic]
    pub children: Vec<StackChild>,
}

/// A child of a stack element.
#[derive(Clone, PartialEq, Hash)]
pub enum StackChild {
    /// Spacing between other children.
    Spacing(Spacing),
    /// Arbitrary block-level content.
    Block(Content),
}

impl Debug for StackChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(kind) => kind.fmt(f),
            Self::Block(block) => block.fmt(f),
        }
    }
}

cast! {
    StackChild,
    self => match self {
        Self::Spacing(spacing) => spacing.into_value(),
        Self::Block(content) => content.into_value(),
    },
    v: Spacing => Self::Spacing(v),
    v: Content => Self::Block(v),
}

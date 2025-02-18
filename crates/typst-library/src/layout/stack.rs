use std::fmt::{self, Debug, Formatter};

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Content, NativeElement, Packed, Show, Smart, StyleChain,
};
use crate::layout::{Alignment, BlockElem, Dir, Spacing};

/// Arranges content and spacing horizontally or vertically.
///
/// The stack places a list of items along an axis, with optional spacing
/// between each item.
///
/// # Example
/// ```example
/// #stack(
///   dir: ttb,
///   rect(width: 40pt),
///   rect(width: 120pt),
///   rect(width: 90pt),
/// )
/// ```
#[elem(Show)]
pub struct StackElem {
    /// The direction along which the items are stacked. Possible values are:
    ///
    /// - `{ltr}`: Left to right.
    /// - `{rtl}`: Right to left.
    /// - `{ttb}`: Top to bottom.
    /// - `{btt}`: Bottom to top.
    ///
    /// You can use the `start` and `end` methods to obtain the initial and
    /// final points (respectively) of a direction, as `alignment`. You can also
    /// use the `axis` method to determine whether a direction is
    /// `{"horizontal"}` or `{"vertical"}`. The `inv` method returns a
    /// direction's inverse direction.
    ///
    /// For example, `{ttb.start()}` is `top`, `{ttb.end()}` is `bottom`,
    /// `{ttb.axis()}` is `{"vertical"}` and `{ttb.inv()}` is equal to `btt`.
    #[default(Dir::TTB)]
    pub dir: Dir,

    /// Spacing to insert between items where no explicit spacing was provided.
    pub spacing: Option<Spacing>,

    /// How to align the items.
    ///
    /// If set to `{auto}`, the outer alignment is used.
    pub align: Smart<Alignment>,

    /// The children to stack along the axis.
    #[variadic]
    pub children: Vec<StackChild>,
}

impl Show for Packed<StackElem> {
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::multi_layouter(self.clone(), engine.routines.layout_stack)
            .pack()
            .spanned(self.span()))
    }
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

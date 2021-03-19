use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use fontdock::{FallbackTree, FontVariant};

use super::*;
use crate::shaping::{shape, VerticalFontMetric};

/// A text node.
#[derive(Clone, PartialEq)]
pub struct NodeText {
    /// The text.
    pub text: String,
    /// The text direction.
    pub dir: Dir,
    /// How to align this text node in its parent.
    pub aligns: LayoutAligns,
    /// The families used for font fallback.
    pub families: Rc<FallbackTree>,
    /// The font variant,
    pub variant: FontVariant,
    /// The font size.
    pub font_size: Length,
    /// The top end of the text bounding box.
    pub top_edge: VerticalFontMetric,
    /// The bottom end of the text bounding box.
    pub bottom_edge: VerticalFontMetric,
}

impl Layout for NodeText {
    fn layout(&self, ctx: &mut LayoutContext, _: &Areas) -> Layouted {
        Layouted::Frame(
            shape(
                &self.text,
                self.dir,
                &self.families,
                self.variant,
                self.font_size,
                self.top_edge,
                self.bottom_edge,
                &mut ctx.env.fonts,
            ),
            self.aligns,
        )
    }
}

impl Debug for NodeText {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({})", self.text)
    }
}

impl From<NodeText> for Node {
    fn from(text: NodeText) -> Self {
        Self::Text(text)
    }
}

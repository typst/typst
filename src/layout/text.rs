use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use fontdock::{FallbackTree, FontVariant};

use super::*;
use crate::shaping;

/// A text node.
#[derive(Clone, PartialEq)]
pub struct NodeText {
    /// The text.
    pub text: String,
    /// How to align this text node in its parent.
    pub align: ChildAlign,
    /// The text direction.
    pub dir: Dir,
    /// The font size.
    pub font_size: Length,
    /// The families used for font fallback.
    pub families: Rc<FallbackTree>,
    /// The font variant,
    pub variant: FontVariant,
}

impl Layout for NodeText {
    fn layout(&self, ctx: &mut LayoutContext, _: &Areas) -> Layouted {
        Layouted::Frame(
            shaping::shape(
                &self.text,
                self.dir,
                self.font_size,
                &mut ctx.env.fonts,
                &self.families,
                self.variant,
            ),
            self.align,
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

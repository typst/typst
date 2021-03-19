use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use fontdock::{FallbackTree, FontVariant};

use super::*;

/// A consecutive, styled run of text.
#[derive(Clone, PartialEq)]
pub struct TextNode {
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
    /// The glyph fill.
    pub color: Fill,
}

impl Layout for TextNode {
    fn layout(&self, ctx: &mut LayoutContext, _: &Areas) -> Fragment {
        Fragment::Frame(
            shape(
                &self.text,
                self.dir,
                &self.families,
                self.variant,
                self.font_size,
                self.top_edge,
                self.bottom_edge,
                self.color,
                &mut ctx.env.fonts,
            ),
            self.aligns,
        )
    }
}

impl Debug for TextNode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({})", self.text)
    }
}

impl From<TextNode> for Node {
    fn from(text: TextNode) -> Self {
        Self::Text(text)
    }
}

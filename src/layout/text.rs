use std::fmt::{self, Debug, Formatter};

use super::*;
use crate::exec::FontProps;

/// A consecutive, styled run of text.
#[derive(Clone, PartialEq)]
pub struct TextNode {
    /// The text direction.
    pub dir: Dir,
    /// How to align this text node in its parent.
    pub aligns: LayoutAligns,
    /// The text.
    pub text: String,
    /// Properties used for font selection and layout.
    pub props: FontProps,
}

impl Layout for TextNode {
    fn layout(&self, ctx: &mut LayoutContext, _: &Areas) -> Fragment {
        let frame = shape(&self.text, &mut ctx.env.fonts, &self.props);
        Fragment::Frame(frame, self.aligns)
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

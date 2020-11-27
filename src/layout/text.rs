use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use fontdock::{FallbackTree, FontVariant};

use super::*;
use crate::shaping;

/// A text node.
#[derive(Clone, PartialEq)]
pub struct Text {
    /// The text.
    pub text: String,
    /// How to align this text node in its parent.
    pub align: BoxAlign,
    /// The text direction.
    pub dir: Dir,
    /// The font size.
    pub font_size: Length,
    /// The families used for font fallback.
    pub families: Rc<FallbackTree>,
    /// The font variant,
    pub variant: FontVariant,
}

impl Layout for Text {
    fn layout(&self, ctx: &mut LayoutContext, _: &Areas) -> Layouted {
        let mut env = ctx.env.borrow_mut();
        Layouted::Layout(
            shaping::shape(
                &mut env.fonts,
                &self.text,
                self.dir,
                self.font_size,
                &self.families,
                self.variant,
            ),
            self.align,
        )
    }
}

impl Debug for Text {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Text({})", self.text)
    }
}

impl From<Text> for LayoutNode {
    fn from(text: Text) -> Self {
        Self::Text(text)
    }
}

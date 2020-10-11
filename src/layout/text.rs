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
    /// The font size.
    pub font_size: Length,
    /// The text direction.
    pub dir: Dir,
    /// The families used for font fallback.
    pub families: Rc<FallbackTree>,
    /// The font variant,
    pub variant: FontVariant,
    /// How to align this text node in its parent.
    pub aligns: Gen<Align>,
}

#[async_trait(?Send)]
impl Layout for Text {
    async fn layout(&self, ctx: &mut LayoutContext, _: &Areas) -> Vec<Layouted> {
        let mut loader = ctx.loader.borrow_mut();
        vec![Layouted::Boxed(
            shaping::shape(
                &mut loader,
                &self.text,
                self.font_size,
                self.dir,
                &self.families,
                self.variant,
            )
            .await,
            self.aligns,
        )]
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

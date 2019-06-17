//! The layouting engine.

use crate::font::{FontLoader, FontError};
use crate::size::{Size2D, SizeBox};
use crate::syntax::{SyntaxTree, Node};
use crate::style::TextStyle;

mod boxed;
mod flex;

pub use flex::{FlexLayout, FlexLayouter};
pub use boxed::{BoxLayout, BoxLayouter};


/// Types that layout components and can be finished into some kind of layout.
pub trait Layouter {
    type Layout;

    /// Finish the layouting and create the layout from this.
    fn finish(self) -> Self::Layout;

    /// Whether this layouter contains any items.
    fn is_empty(&self) -> bool;
}

/// A collection of layouted content.
#[derive(Debug, Clone)]
pub enum Layout {
    /// A box layout.
    Boxed(BoxLayout),
    /// A flexible layout.
    Flex(FlexLayout),
}

/// The context for layouting.
#[derive(Debug, Clone)]
pub struct LayoutContext<'a, 'p> {
    /// Loads fonts matching queries.
    pub loader: &'a FontLoader<'p>,
    /// Base style to set text with.
    pub style: TextStyle,
    /// The space to layout in.
    pub space: LayoutSpace,
}

/// Spacial constraints for layouting.
#[derive(Debug, Clone)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub dimensions: Size2D,
    /// Padding that should be respected on each side.
    pub padding: SizeBox,
}

/// Layout a syntax tree in a given context.
pub fn layout(tree: &SyntaxTree, ctx: &LayoutContext) -> LayoutResult<BoxLayout> {
    // The top-level layouter and the sub-level layouter.
    let mut box_layouter = BoxLayouter::new(ctx);
    let mut flex_layouter = FlexLayouter::new(ctx);

    // The current text style.
    let mut italic = false;
    let mut bold = false;

    // Walk all nodes and layout them.
    for node in &tree.nodes {
        match node {
            Node::Text(text) => {
                unimplemented!()
            },
            Node::Space => {
                unimplemented!()
            },
            Node::Newline => {
                unimplemented!()
            },

            // Toggle the text styles.
            Node::ToggleItalics => italic = !italic,
            Node::ToggleBold => bold = !bold,

            Node::Func(func) => {
                unimplemented!()
            }
        }
    }

    // If there are remainings, add them to the layout.
    if !flex_layouter.is_empty() {
        let boxed = flex_layouter.finish().into_box();
        box_layouter.add_box(boxed);
    }

    Ok(box_layouter.finish())
}

/// The error type for layouting.
pub enum LayoutError {
    /// There was no suitable font for the given character.
    NoSuitableFont(char),
    /// An error occured while gathering font data.
    Font(FontError),
}

/// The result type for layouting.
pub type LayoutResult<T> = Result<T, LayoutError>;

error_type! {
    err: LayoutError,
    show: f => match err {
        LayoutError::NoSuitableFont(c) => write!(f, "no suitable font for '{}'", c),
        LayoutError::Font(err) => write!(f, "font error: {}", err),
    },
    source: match err {
        LayoutError::Font(err) => Some(err),
        _ => None,
    },
    from: (std::io::Error, LayoutError::Font(FontError::Io(err))),
    from: (FontError, LayoutError::Font(err)),
}

//! The layouting engine.

use crate::doc::TextAction;
use crate::font::{FontLoader, FontError};
use crate::size::{Size, Size2D, SizeBox};
use crate::syntax::{SyntaxTree, Node};
use crate::style::TextStyle;

use self::flex::{FlexLayout, FlexContext};
use self::boxed::{BoxLayout, BoxContext, BoxLayouter};
use self::text::TextContext;

pub mod text;
pub mod boxed;
pub mod flex;


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
#[derive(Debug, Copy, Clone)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub dimensions: Size2D,
    /// Padding that should be respected on each side.
    pub padding: SizeBox,
    /// Whether to shrink the dimensions to fit the content or the keep the
    /// original ones.
    pub shrink_to_fit: bool,
}

impl LayoutSpace {
    /// The actually usable area.
    pub fn usable(&self) -> Size2D {
        Size2D {
            x: self.dimensions.x - self.padding.left - self.padding.right,
            y: self.dimensions.y - self.padding.top - self.padding.bottom,
        }
    }
}

/// Layout a syntax tree in a given context.
pub fn layout(tree: &SyntaxTree, ctx: &LayoutContext) -> LayoutResult<BoxLayout> {
    Layouter::new(tree, ctx).layout()
}

/// Transforms a syntax tree into a box layout.
#[derive(Debug)]
struct Layouter<'a, 'p> {
    tree: &'a SyntaxTree,
    box_layouter: BoxLayouter,
    flex_layout: FlexLayout,
    flex_ctx: FlexContext,
    text_ctx: TextContext<'a, 'p>,
}

impl<'a, 'p> Layouter<'a, 'p> {
    /// Create a new layouter.
    fn new(tree: &'a SyntaxTree, ctx: &LayoutContext<'a, 'p>) -> Layouter<'a, 'p> {
        // The top-level context for arranging paragraphs.
        let box_ctx = BoxContext { space: ctx.space };

        // The sub-level context for arranging pieces of text.
        let flex_ctx = FlexContext {
            space: LayoutSpace {
                dimensions: ctx.space.usable(),
                padding: SizeBox::zero(),
                shrink_to_fit: true,
            },
            flex_spacing: ctx.style.line_spacing,
        };

        // The mutable context for layouting single pieces of text.
        let text_ctx = TextContext {
            loader: &ctx.loader,
            style: ctx.style.clone(),
        };

        Layouter {
            tree,
            box_layouter: BoxLayouter::new(box_ctx),
            flex_layout: FlexLayout::new(flex_ctx),
            flex_ctx,
            text_ctx,
        }
    }

    /// Layout the tree into a box.
    fn layout(mut self) -> LayoutResult<BoxLayout> {
        // Walk all nodes and layout them.
        for node in &self.tree.nodes {
            match node {
                // Layout a single piece of text.
                Node::Text(text) => {
                    let boxed = self::text::layout(text, &self.text_ctx)?;
                    self.flex_layout.add_box(boxed);
                },
                Node::Space => {
                    if !self.flex_layout.is_empty() {
                        let boxed = self::text::layout(" ", &self.text_ctx)?;
                        self.flex_layout.add_glue(boxed);
                    }
                },

                // Finish the current flex layout and add it to the box layouter.
                // Then start a new flex layouting process.
                Node::Newline => {
                    // Finish the current paragraph into a box and add it.
                    self.add_paragraph_spacing()?;
                    let boxed = self.flex_layout.into_box()?;
                    self.box_layouter.add_box(boxed)?;

                    // Create a fresh flex layout for the next paragraph.
                    self.flex_ctx.space.dimensions = self.box_layouter.remaining();
                    self.flex_layout = FlexLayout::new(self.flex_ctx);
                },

                // Toggle the text styles.
                Node::ToggleItalics => self.text_ctx.style.italic = !self.text_ctx.style.italic,
                Node::ToggleBold => self.text_ctx.style.bold = !self.text_ctx.style.bold,

                // Execute a function.
                Node::Func(_) => unimplemented!(),
            }
        }

        // If there are remainings, add them to the layout.
        if !self.flex_layout.is_empty() {
            self.add_paragraph_spacing()?;
            let boxed = self.flex_layout.into_box()?;
            self.box_layouter.add_box(boxed)?;
        }

        Ok(self.box_layouter.finish())
    }

    /// Add the spacing between two paragraphs.
    fn add_paragraph_spacing(&mut self) -> LayoutResult<()> {
        let size = Size::points(self.text_ctx.style.font_size)
            * (self.text_ctx.style.line_spacing * self.text_ctx.style.paragraph_spacing - 1.0);
        self.box_layouter.add_space(size)
    }
}

/// Manipulates and optimizes a list of actions.
#[derive(Debug, Clone)]
pub struct ActionList {
    actions: Vec<TextAction>,
    origin: Size2D,
    active_font: (usize, f32),
}

impl ActionList {
    /// Create a new action list.
    pub fn new() -> ActionList {
        ActionList {
            actions: vec![],
            origin: Size2D::zero(),
            active_font: (std::usize::MAX, 0.0),
        }
    }

    /// Add an action to the list if it is not useless
    /// (like changing to a font that is already active).
    pub fn add(&mut self, action: TextAction) {
        use TextAction::*;
        match action {
            MoveAbsolute(pos) => self.actions.push(MoveAbsolute(self.origin + pos)),
            SetFont(index, size) => if (index, size) != self.active_font {
                self.active_font = (index, size);
                self.actions.push(action);
            },
            _ => self.actions.push(action),
        }
    }

    /// Add a series of actions.
    pub fn extend<I>(&mut self, actions: I) where I: IntoIterator<Item=TextAction> {
        for action in actions.into_iter() {
            self.add(action);
        }
    }

    /// Move the origin for the upcomming actions. Absolute moves will be
    /// changed by that origin.
    pub fn set_origin(&mut self, origin: Size2D) {
        self.origin = origin;
    }

    /// Reset the origin to zero.
    pub fn reset_origin(&mut self) {
        self.origin = Size2D::zero();
    }

    /// Whether there are any actions in this list.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Return the list of actions as a vector.
    pub fn into_vec(self) -> Vec<TextAction> {
        self.actions
    }
}

/// The error type for layouting.
pub enum LayoutError {
    /// There is not enough space to add an item.
    NotEnoughSpace,
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
        LayoutError::NotEnoughSpace => write!(f, "not enough space"),
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

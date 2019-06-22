//! The layouting engine.

use std::borrow::Cow;
use std::mem;

use crate::doc::LayoutAction;
use crate::font::{FontLoader, FontClass, FontError};
use crate::size::{Size, Size2D, SizeBox};
use crate::syntax::{SyntaxTree, Node, FuncCall};
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

/// Layout a syntax tree in a given context.
pub fn layout(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<BoxLayout> {
    Layouter::new(tree, ctx).layout()
}

/// The context for layouting.
#[derive(Debug, Copy, Clone)]
pub struct LayoutContext<'a, 'p> {
    /// Loads fonts matching queries.
    pub loader: &'a FontLoader<'p>,
    /// Base style to set text with.
    pub style: &'a TextStyle,
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

/// Transforms a syntax tree into a box layout.
#[derive(Debug)]
struct Layouter<'a, 'p> {
    tree: &'a SyntaxTree,
    box_layouter: BoxLayouter,
    flex_layout: FlexLayout,
    loader: &'a FontLoader<'p>,
    style: Cow<'a, TextStyle>,
}

impl<'a, 'p> Layouter<'a, 'p> {
    /// Create a new layouter.
    fn new(tree: &'a SyntaxTree, ctx: LayoutContext<'a, 'p>) -> Layouter<'a, 'p> {
        Layouter {
            tree,
            box_layouter: BoxLayouter::new(BoxContext { space: ctx.space }),
            flex_layout: FlexLayout::new(),
            loader: ctx.loader,
            style: Cow::Borrowed(ctx.style)
        }
    }

    /// Layout the tree into a box.
    fn layout(mut self) -> LayoutResult<BoxLayout> {
        // Walk all nodes and layout them.
        for node in &self.tree.nodes {
            match node {
                // Layout a single piece of text.
                Node::Text(text) => self.layout_text(text, false)?,

                // Add a space.
                Node::Space => {
                    if !self.flex_layout.is_empty() {
                        self.layout_text(" ", true)?;
                    }
                },

                // Finish the current flex layout and add it to the box layouter.
                Node::Newline => {
                    // Finish the current paragraph into a box and add it.
                    self.layout_flex()?;

                    // Add some paragraph spacing.
                    let size = Size::pt(self.style.font_size)
                        * (self.style.line_spacing * self.style.paragraph_spacing - 1.0);
                    self.box_layouter.add_space(size)?;
                },

                // Toggle the text styles.
                Node::ToggleItalics => self.style.to_mut().toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.to_mut().toggle_class(FontClass::Bold),

                // Execute a function.
                Node::Func(func) => self.layout_func(func)?,
            }
        }

        // If there are remainings, add them to the layout.
        if !self.flex_layout.is_empty() {
            self.layout_flex()?;
        }

        Ok(self.box_layouter.finish())
    }

    /// Layout a piece of text into a box.
    fn layout_text(&mut self, text: &str, glue: bool) -> LayoutResult<()> {
        let boxed = self::text::layout(text, TextContext {
            loader: &self.loader,
            style: &self.style,
        })?;

        if glue {
            self.flex_layout.add_glue(boxed);
        } else {
            self.flex_layout.add_box(boxed);
        }

        Ok(())
    }

    /// Finish the current flex run and return the resulting box.
    fn layout_flex(&mut self) -> LayoutResult<()> {
        let mut layout = FlexLayout::new();
        mem::swap(&mut layout, &mut self.flex_layout);

        let boxed = layout.finish(FlexContext {
            space: LayoutSpace {
                dimensions: self.box_layouter.remaining(),
                padding: SizeBox::zero(),
                shrink_to_fit: true,
            },
            flex_spacing: (self.style.line_spacing - 1.0) * Size::pt(self.style.font_size),
        })?;

        self.box_layouter.add_box(boxed)
    }

    /// Layout a function.
    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let layout = func.body.layout(LayoutContext {
            loader: &self.loader,
            style: &self.style,
            space: LayoutSpace {
                dimensions: self.box_layouter.remaining(),
                padding: SizeBox::zero(),
                shrink_to_fit: true,
            },
        })?;

        // Add the potential layout.
        if let Some(layout) = layout {
            match layout {
                Layout::Boxed(boxed) => {
                    // Finish the previous flex run before adding the box.
                    self.layout_flex()?;
                    self.box_layouter.add_box(boxed)?;
                },
                Layout::Flex(flex) => self.flex_layout.add_flexible(flex),
            }
        }

        Ok(())
    }
}

/// Manipulates and optimizes a list of actions.
#[derive(Debug, Clone)]
pub struct ActionList {
    actions: Vec<LayoutAction>,
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
    pub fn add(&mut self, action: LayoutAction) {
        use LayoutAction::*;
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
    pub fn extend<I>(&mut self, actions: I) where I: IntoIterator<Item=LayoutAction> {
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
    pub fn into_vec(self) -> Vec<LayoutAction> {
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

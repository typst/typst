//! The core layouting engine.

use std::borrow::Cow;
use std::io::{self, Write};
use std::mem;

use toddle::query::{FontClass, SharedFontLoader};
use toddle::Error as FontError;

use crate::func::Command;
use crate::size::{Size, Size2D, SizeBox};
use crate::style::TextStyle;
use crate::syntax::{FuncCall, Node, SyntaxTree};

mod actions;
mod flex;
mod stacked;
mod text;

pub use actions::{LayoutAction, LayoutActionList};
pub use flex::{FlexContext, FlexLayouter};
pub use stacked::{StackContext, StackLayouter};
pub use text::{layout_text, TextContext};

/// A box layout has a fixed width and height and composes of actions.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The size of the box.
    pub dimensions: Size2D,
    /// The actions composing this layout.
    pub actions: Vec<LayoutAction>,
    /// Whether to debug-render this box.
    pub debug_render: bool,
}

impl Layout {
    /// Serialize this layout into an output buffer.
    pub fn serialize<W: Write>(&self, f: &mut W) -> io::Result<()> {
        writeln!(
            f,
            "{:.4} {:.4}",
            self.dimensions.x.to_pt(),
            self.dimensions.y.to_pt()
        )?;
        for action in &self.actions {
            action.serialize(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

/// A collection of box layouts.
#[derive(Debug, Clone)]
pub struct MultiLayout {
    pub layouts: Vec<Layout>,
}

impl MultiLayout {
    /// Create an empty multibox layout.
    pub fn new() -> MultiLayout {
        MultiLayout { layouts: vec![] }
    }

    /// Extract a single sublayout and panic if this layout does not have
    /// exactly one child.
    pub fn into_single(mut self) -> Layout {
        if self.layouts.len() != 1 {
            panic!("into_single: contains not exactly one layout");
        }
        self.layouts.pop().unwrap()
    }

    /// Add a sublayout.
    pub fn add(&mut self, layout: Layout) {
        self.layouts.push(layout);
    }

    /// Whether this layout contains any sublayouts.
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}

impl IntoIterator for MultiLayout {
    type Item = Layout;
    type IntoIter = std::vec::IntoIter<Layout>;

    fn into_iter(self) -> Self::IntoIter {
        self.layouts.into_iter()
    }
}

/// The context for layouting.
#[derive(Copy, Clone)]
pub struct LayoutContext<'a, 'p> {
    pub loader: &'a SharedFontLoader<'p>,
    pub style: &'a TextStyle,
    pub space: LayoutSpace,
    pub extra_space: Option<LayoutSpace>,
}

/// Spacial constraints for layouting.
#[derive(Debug, Copy, Clone)]
pub struct LayoutSpace {
    /// The maximum size of the box to layout in.
    pub dimensions: Size2D,
    /// Padding that should be respected on each side.
    pub padding: SizeBox,
    /// The alignment to use for the content.
    pub alignment: Alignment,
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

/// Where to align content.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Alignment {
    Left,
    Right,
}

pub fn layout_tree(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = Layouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

/// Transforms a syntax tree into a box layout.
struct Layouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    stack_layouter: StackLayouter,
    flex_layouter: FlexLayouter,
    style: Cow<'a, TextStyle>,
}

impl<'a, 'p> Layouter<'a, 'p> {
    /// Create a new layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> Layouter<'a, 'p> {
        Layouter {
            ctx,
            stack_layouter: StackLayouter::new(StackContext { space: ctx.space }),
            flex_layouter: FlexLayouter::new(FlexContext {
                space: LayoutSpace {
                    dimensions: ctx.space.usable(),
                    padding: SizeBox::zero(),
                    alignment: ctx.space.alignment,
                    shrink_to_fit: true,
                },
                flex_spacing: (ctx.style.line_spacing - 1.0) * Size::pt(ctx.style.font_size),
            }),
            style: Cow::Borrowed(ctx.style),
        }
    }

    /// Layout the tree into a box.
    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        // Walk all nodes and layout them.
        for node in &tree.nodes {
            match node {
                // Layout a single piece of text.
                Node::Text(text) => self.layout_text(text, false)?,

                // Add a space.
                Node::Space => {
                    if !self.flex_layouter.is_empty() {
                        self.layout_text(" ", true)?;
                    }
                }

                // Finish the current flex layout and add it to the box layouter.
                Node::Newline => {
                    // Finish the current paragraph into a box and add it.
                    self.layout_flex()?;

                    // Add some paragraph spacing.
                    let size = Size::pt(self.style.font_size)
                        * (self.style.line_spacing * self.style.paragraph_spacing - 1.0);
                    self.stack_layouter.add_space(size)?;
                }

                // Toggle the text styles.
                Node::ToggleItalics => self.style.to_mut().toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.to_mut().toggle_class(FontClass::Bold),
                Node::ToggleMonospace => self.style.to_mut().toggle_class(FontClass::Monospace),

                // Execute a function.
                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    fn finish(mut self) -> LayoutResult<MultiLayout> {
        // If there are remainings, add them to the layout.
        if !self.flex_layouter.is_empty() {
            self.layout_flex()?;
        }

        Ok(MultiLayout {
            layouts: vec![self.stack_layouter.finish()],
        })
    }

    /// Layout a piece of text into a box.
    fn layout_text(&mut self, text: &str, glue: bool) -> LayoutResult<()> {
        let boxed = layout_text(
            text,
            TextContext {
                loader: &self.ctx.loader,
                style: &self.style,
            },
        )?;

        if glue {
            self.flex_layouter.add_glue(boxed);
        } else {
            self.flex_layouter.add(boxed);
        }

        Ok(())
    }

    /// Finish the current flex run and return the resulting box.
    fn layout_flex(&mut self) -> LayoutResult<()> {
        if self.flex_layouter.is_empty() {
            return Ok(());
        }

        let mut layout = FlexLayouter::new(FlexContext {
            space: LayoutSpace {
                dimensions: self.stack_layouter.ctx().space.usable(),
                padding: SizeBox::zero(),
                alignment: self.ctx.space.alignment,
                shrink_to_fit: true,
            },
            flex_spacing: (self.style.line_spacing - 1.0) * Size::pt(self.style.font_size),
        });
        mem::swap(&mut layout, &mut self.flex_layouter);

        let boxed = layout.finish()?;

        self.stack_layouter.add_box(boxed)
    }

    /// Layout a function.
    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let commands = func.body.layout(LayoutContext {
            loader: &self.ctx.loader,
            style: &self.style,
            space: LayoutSpace {
                dimensions: self.stack_layouter.remaining(),
                padding: SizeBox::zero(),
                alignment: self.ctx.space.alignment,
                shrink_to_fit: true,
            },
            extra_space: self.ctx.extra_space,
        })?;

        for command in commands {
            match command {
                Command::Layout(tree) => self.layout(tree)?,
                Command::Add(layout) => self.stack_layouter.add_box(layout)?,
                Command::AddMany(layouts) => self.stack_layouter.add_many(layouts)?,
                Command::ToggleStyleClass(class) => self.style.to_mut().toggle_class(class),
            }
        }

        Ok(())
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

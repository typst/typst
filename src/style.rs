//! Styles for text and pages.

use toddle::query::FontClass;
use FontClass::*;

use crate::size::{Size, Size2D, SizeBox};

/// Defines properties of pages and text.
#[derive(Debug, Default, Clone)]
pub struct LayoutStyle {
    pub page: PageStyle,
    pub text: TextStyle,
}

/// Defines which fonts to use and how to space text.
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// The classes the font has to be part of.
    pub classes: Vec<FontClass>,
    /// The fallback classes from which the font needs to match the
    /// leftmost possible one.
    pub fallback: Vec<FontClass>,
    /// The font size.
    pub font_size: Size,
    /// The word spacing (as a multiple of the font size).
    pub word_spacing: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The paragraphs spacing (as a multiple of the font size).
    pub paragraph_spacing: f32,
}

impl TextStyle {
    /// Toggle a class.
    ///
    /// If the class was one of _italic_ or _bold_, then:
    /// - If it was not present before, the _regular_ class will be removed.
    /// - If it was present before, the _regular_ class will be added in case the other
    ///   style class is not present.
    pub fn toggle_class(&mut self, class: FontClass) {
        if self.classes.contains(&class) {
            // If we retain a Bold or Italic class, we will not add
            // the Regular class.
            let mut regular = true;
            self.classes.retain(|x| {
                if class == *x {
                    false
                } else {
                    if class == Bold || class == Italic {
                        regular = false;
                    }
                    true
                }
            });

            if regular {
                self.classes.push(Regular);
            }
        } else {
            // If we add an Italic or Bold class, we remove
            // the Regular class.
            if class == Italic || class == Bold {
                self.classes.retain(|x| x != &Regular);
            }

            self.classes.push(class);
        }
    }
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle {
            classes: vec![Regular],
            fallback: vec![Serif],
            font_size: Size::pt(11.0),
            word_spacing: 0.25,
            line_spacing: 1.2,
            paragraph_spacing: 1.5,
        }
    }
}

/// Defines the size and margins of a page.
#[derive(Debug, Copy, Clone)]
pub struct PageStyle {
    /// The width and height of the page.
    pub dimensions: Size2D,
    /// The amount of white space on each side.
    pub margins: SizeBox,
}

impl Default for PageStyle {
    fn default() -> PageStyle {
        PageStyle {
            // A4 paper.
            dimensions: Size2D {
                x: Size::mm(210.0),
                y: Size::mm(297.0),
            },

            // All the same margins.
            margins: SizeBox {
                left: Size::cm(2.5),
                top: Size::cm(2.5),
                right: Size::cm(2.5),
                bottom: Size::cm(2.5),
            },
        }
    }
}

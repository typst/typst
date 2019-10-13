//! Layouting styles.

use toddle::query::FontClass;

use crate::size::{Size, Size2D, SizeBox};


/// Default styles for text.
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// The classes the font we want has to be part of.
    pub classes: Vec<FontClass>,
    /// A sequence of classes. We need the font to be part of at least one of these
    /// and preferably the leftmost possible.
    pub fallback: Vec<FontClass>,
    /// The font size.
    pub font_size: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The paragraphs spacing (as a multiple of the line spacing).
    pub paragraph_spacing: f32,
}

impl TextStyle {
    /// Toggle a class.
    ///
    /// If the class was one of _italic_ or _bold_, then:
    /// - If it was not present, the _regular_ class will be removed.
    /// - If it was present, the _regular_ class will be added in case the
    ///   other style class is not present.
    pub fn toggle_class(&mut self, class: FontClass) {
        if self.classes.contains(&class) {
            self.classes.retain(|x| x != &class);
            if (class == FontClass::Italic && !self.classes.contains(&FontClass::Bold))
               || (class == FontClass::Bold && !self.classes.contains(&FontClass::Italic)) {
                self.classes.push(FontClass::Regular);
            }
        } else {
            if class == FontClass::Italic || class == FontClass::Bold {
                self.classes.retain(|x| x != &FontClass::Regular);
            }
            self.classes.push(class);
        }
    }
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        use FontClass::*;
        TextStyle {
            classes: vec![Regular],
            fallback: vec![Serif],
            font_size: 11.0,
            line_spacing: 1.2,
            paragraph_spacing: 1.5,
        }
    }
}

/// Default styles for pages.
#[derive(Debug, Clone)]
pub struct PageStyle {
    /// Width and height of the page.
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

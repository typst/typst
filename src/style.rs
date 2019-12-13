//! Styles for text and pages.

use toddle::query::FontClass;
use FontClass::*;

use crate::size::{Size, Size2D, SizeBox};
use crate::syntax::ParseResult;

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
    /// The base font size.
    pub base_font_size: Size,
    /// The font scale to apply on the base font size.
    pub font_scale: f32,
    /// The word spacing (as a multiple of the font size).
    pub word_spacing: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The paragraphs spacing (as a multiple of the font size).
    pub paragraph_spacing: f32,
}

impl TextStyle {
    /// The scaled font size.
    pub fn font_size(&self) -> Size {
        self.base_font_size * self.font_scale
    }

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
            base_font_size: Size::pt(11.0),
            font_scale: 1.0,
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

/// The size of a page with the given name.
pub fn parse_paper_name(paper: &str) -> ParseResult<Size2D> {
    Ok(match paper {
        "A0" | "a0" => PAPER_A0,
        "A1" | "a1" => PAPER_A1,
        "A2" | "a2" => PAPER_A2,
        "A3" | "a3" => PAPER_A3,
        "A4" | "a4" => PAPER_A4,
        "A5" | "a5" => PAPER_A5,
        "A6" | "a6" => PAPER_A6,
        "A7" | "a7" => PAPER_A7,
        "A8" | "a8" => PAPER_A8,
        "A9" | "a9" => PAPER_A9,
        "A10" | "a10" => PAPER_A10,
        "A11" | "a11" => PAPER_A11,
        "Letter" | "letter" => PAPER_LETTER,
        "Legal" | "legal" => PAPER_LEGAL,
        "Tabloid" | "tabloid" => PAPER_TABLOID,
        "Ledger" | "ledger" => PAPER_LEDGER,
        "Junior-Legal" | "junior-legal" => PAPER_JUNIOR_LEGAL,
        "Half-Letter" | "half-letter" => PAPER_HALF_LETTER,
        "Government-Letter" | "government-letter" => PAPER_GOVERNMENT_LETTER,

        _ => error!("unknown paper size: `{}`", paper),
    })
}

macro_rules! paper {
    ($var:ident: $width:expr, $height: expr) => {
        /// The size of the paper that's in the name.
        pub const $var: Size2D = Size2D {
            x: Size { points: 2.83465 * $width },
            y: Size { points: 2.83465 * $height },
        };
    };
}

// Common paper sizes in mm.
paper!(PAPER_A0:  841.0, 1189.0);
paper!(PAPER_A1:  594.0, 841.0);
paper!(PAPER_A2:  420.0, 594.0);
paper!(PAPER_A3:  297.0, 420.0);
paper!(PAPER_A4:  210.0, 297.0);
paper!(PAPER_A5:  148.0, 210.0);
paper!(PAPER_A6:  105.0, 148.0);
paper!(PAPER_A7:  74.0,  105.0);
paper!(PAPER_A8:  52.0,  74.0);
paper!(PAPER_A9:  37.0,  52.0);
paper!(PAPER_A10: 26.0,  37.0);
paper!(PAPER_A11: 18.0,  26.0);
paper!(PAPER_LETTER:            216.0, 279.0);
paper!(PAPER_LEGAL:             216.0, 356.0);
paper!(PAPER_TABLOID:           279.0, 432.0);
paper!(PAPER_LEDGER:            432.0, 279.0);
paper!(PAPER_JUNIOR_LEGAL:      127.0, 203.0);
paper!(PAPER_HALF_LETTER:       140.0, 216.0);
paper!(PAPER_GOVERNMENT_LETTER: 203.0, 267.0);

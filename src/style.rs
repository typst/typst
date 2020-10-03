//! Styles for text and pages.

use fontdock::{fallback, FallbackTree, FontStretch, FontStyle, FontVariant, FontWeight};

use crate::geom::{Margins, Size, Value4};
use crate::length::{Length, ScaleLength};
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// Defines properties of pages and text.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LayoutStyle {
    /// The style for text.
    pub text: TextStyle,
    /// The style for pages.
    pub page: PageStyle,
}

/// Defines which fonts to use and how to space text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// A tree of font family names and generic class names.
    pub fallback: FallbackTree,
    /// The selected font variant.
    pub variant: FontVariant,
    /// Whether the strong toggle is active or inactive. This determines
    /// whether the next `*` adds or removes font weight.
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. This determines
    /// whether the next `_` makes italic or non-italic.
    pub emph: bool,
    /// The base font size.
    pub base_font_size: f64,
    /// The font scale to apply on the base font size.
    pub font_scale: f64,
    /// The word spacing (as a multiple of the font size).
    pub word_spacing_scale: f64,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing_scale: f64,
    /// The paragraphs spacing (as a multiple of the font size).
    pub paragraph_spacing_scale: f64,
}

impl TextStyle {
    /// The scaled font size.
    pub fn font_size(&self) -> f64 {
        self.base_font_size * self.font_scale
    }

    /// The absolute word spacing.
    pub fn word_spacing(&self) -> f64 {
        self.word_spacing_scale * self.font_size()
    }

    /// The absolute line spacing.
    pub fn line_spacing(&self) -> f64 {
        (self.line_spacing_scale - 1.0) * self.font_size()
    }

    /// The absolute paragraph spacing.
    pub fn paragraph_spacing(&self) -> f64 {
        (self.paragraph_spacing_scale - 1.0) * self.font_size()
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            fallback: fallback! {
                list: ["sans-serif"],
                classes: {
                    "serif" => ["source serif pro", "noto serif"],
                    "sans-serif" => ["source sans pro", "noto sans"],
                    "monospace" => ["source code pro", "noto sans mono"],
                    "math" => ["latin modern math", "serif"],
                },
                base: [
                    "source sans pro", "noto sans", "segoe ui emoji",
                    "noto emoji", "latin modern math",
                ],
            },
            variant: FontVariant {
                style: FontStyle::Normal,
                weight: FontWeight::REGULAR,
                stretch: FontStretch::Normal,
            },
            strong: false,
            emph: false,
            base_font_size: Length::pt(11.0).as_raw(),
            font_scale: 1.0,
            word_spacing_scale: 0.25,
            line_spacing_scale: 1.2,
            paragraph_spacing_scale: 1.5,
        }
    }
}

/// Defines the size and margins of a page.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PageStyle {
    /// The class of this page.
    pub class: PaperClass,
    /// The width and height of the page.
    pub size: Size,
    /// The amount of white space on each side. If a side is set to `None`, the
    /// default for the paper class is used.
    pub margins: Value4<Option<ScaleLength>>,
}

impl PageStyle {
    /// The default page style for the given paper.
    pub fn new(paper: Paper) -> Self {
        Self {
            class: paper.class,
            size: paper.size(),
            margins: Value4::with_all(None),
        }
    }

    /// The absolute margins.
    pub fn margins(&self) -> Margins {
        let size = self.size;
        let default = self.class.default_margins();
        Margins {
            left: self.margins.left.unwrap_or(default.left).raw_scaled(size.x),
            top: self.margins.top.unwrap_or(default.top).raw_scaled(size.y),
            right: self.margins.right.unwrap_or(default.right).raw_scaled(size.x),
            bottom: self.margins.bottom.unwrap_or(default.bottom).raw_scaled(size.y),
        }
    }
}

impl Default for PageStyle {
    fn default() -> Self {
        Self::new(PAPER_A4)
    }
}

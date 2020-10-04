//! State.

use fontdock::{fallback, FallbackTree, FontStretch, FontStyle, FontVariant, FontWeight};

use super::Scope;
use crate::geom::{Insets, Linear, Sides, Size};
use crate::layout::{Dir, GenAlign, LayoutAlign, LayoutSystem};
use crate::length::Length;
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// The active evaluation state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The scope that contains function definitions.
    pub scope: Scope,
    /// The text state.
    pub text: TextState,
    /// The page state.
    pub page: PageState,
    /// The active layouting system.
    pub sys: LayoutSystem,
    /// The active alignments.
    pub align: LayoutAlign,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scope: crate::library::_std(),
            text: TextState::default(),
            page: PageState::default(),
            sys: LayoutSystem::new(Dir::LTR, Dir::TTB),
            align: LayoutAlign::new(GenAlign::Start, GenAlign::Start),
        }
    }
}

/// Defines which fonts to use and how to space text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextState {
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
    /// The font size.
    pub font_size: FontSize,
    /// The word spacing (relative to the the font size).
    pub word_spacing: Linear,
    /// The line spacing (relative to the the font size).
    pub line_spacing: Linear,
    /// The paragraphs spacing (relative to the the font size).
    pub par_spacing: Linear,
}

impl TextState {
    /// The absolute font size.
    pub fn font_size(&self) -> f64 {
        self.font_size.eval()
    }

    /// The absolute word spacing.
    pub fn word_spacing(&self) -> f64 {
        self.word_spacing.eval(self.font_size())
    }

    /// The absolute line spacing.
    pub fn line_spacing(&self) -> f64 {
        self.line_spacing.eval(self.font_size())
    }

    /// The absolute paragraph spacing.
    pub fn par_spacing(&self) -> f64 {
        self.par_spacing.eval(self.font_size())
    }
}

impl Default for TextState {
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
            font_size: FontSize::abs(Length::pt(11.0).as_raw()),
            word_spacing: Linear::rel(0.25),
            line_spacing: Linear::rel(0.2),
            par_spacing: Linear::rel(0.5),
        }
    }
}

/// The font size, defined by base and scale.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSize {
    /// The base font size, updated whenever the font size is set absolutely.
    pub base: f64,
    /// The scale to apply on the base font size, updated when the font size
    /// is set relatively.
    pub scale: Linear,
}

impl FontSize {
    /// Create a new font size.
    pub fn new(base: f64, scale: Linear) -> Self {
        Self { base, scale }
    }

    /// Create a new font size with the given `base` and a scale of `1.0`.
    pub fn abs(base: f64) -> Self {
        Self::new(base, Linear::rel(1.0))
    }

    /// Compute the absolute font size.
    pub fn eval(&self) -> f64 {
        self.scale.eval(self.base)
    }
}

/// Defines the size and margins of a page.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PageState {
    /// The class of this page.
    pub class: PaperClass,
    /// The width and height of the page.
    pub size: Size,
    /// The amount of white space in the order [left, top, right, bottom]. If a
    /// side is set to `None`, the default for the paper class is used.
    pub margins: Sides<Option<Linear>>,
}

impl PageState {
    /// The default page style for the given paper.
    pub fn new(paper: Paper) -> Self {
        Self {
            class: paper.class,
            size: paper.size(),
            margins: Sides::uniform(None),
        }
    }

    /// The absolute insets.
    pub fn insets(&self) -> Insets {
        let Size { width, height } = self.size;
        let default = self.class.default_margins();
        Insets {
            x0: -self.margins.left.unwrap_or(default.left).eval(width),
            y0: -self.margins.top.unwrap_or(default.top).eval(height),
            x1: -self.margins.right.unwrap_or(default.right).eval(width),
            y1: -self.margins.bottom.unwrap_or(default.bottom).eval(height),
        }
    }
}

impl Default for PageState {
    fn default() -> Self {
        Self::new(PAPER_A4)
    }
}

//! Evaluation state.

use std::rc::Rc;

use fontdock::{fallback, FallbackTree, FontStretch, FontStyle, FontVariant, FontWeight};

use super::Scope;
use crate::geom::{Align, Dir, Gen, Length, Linear, Relative, Sides, Size};
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// The active evaluation state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The scope that contains function definitions.
    pub scope: Scope,
    /// The page state.
    pub page: PageState,
    /// The paragraph state.
    pub par: ParState,
    /// The font state.
    pub font: FontState,
    /// The active layouting directions.
    pub dirs: Gen<Dir>,
    /// The active alignments.
    pub aligns: Gen<Align>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scope: crate::library::_std(),
            page: PageState::default(),
            par: ParState::default(),
            font: FontState::default(),
            dirs: Gen::new(Dir::TTB, Dir::LTR),
            aligns: Gen::new(Align::Start, Align::Start),
        }
    }
}

/// Defines page properties.
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

    /// The margins.
    pub fn margins(&self) -> Sides<Linear> {
        let default = self.class.default_margins();
        Sides {
            left: self.margins.left.unwrap_or(default.left),
            top: self.margins.top.unwrap_or(default.top),
            right: self.margins.right.unwrap_or(default.right),
            bottom: self.margins.bottom.unwrap_or(default.bottom),
        }
    }
}

impl Default for PageState {
    fn default() -> Self {
        Self::new(PAPER_A4)
    }
}

/// Defines paragraph properties.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ParState {
    /// The spacing between words (dependent on scaled font size).
    pub word_spacing: Linear,
    /// The spacing between lines (dependent on scaled font size).
    pub line_spacing: Linear,
    /// The spacing between paragraphs (dependent on scaled font size).
    pub par_spacing: Linear,
}

impl Default for ParState {
    fn default() -> Self {
        Self {
            word_spacing: Relative::new(0.25).into(),
            line_spacing: Relative::new(0.2).into(),
            par_spacing: Relative::new(0.5).into(),
        }
    }
}

/// Defines font properties.
#[derive(Debug, Clone, PartialEq)]
pub struct FontState {
    /// A tree of font family names and generic class names.
    pub families: Rc<FallbackTree>,
    /// The selected font variant.
    pub variant: FontVariant,
    /// The font size.
    pub size: Length,
    /// The linear to apply on the base font size.
    pub scale: Linear,
    /// Whether the strong toggle is active or inactive. This determines
    /// whether the next `*` adds or removes font weight.
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. This determines
    /// whether the next `_` makes italic or non-italic.
    pub emph: bool,
}

impl FontState {
    /// The absolute font size.
    pub fn font_size(&self) -> Length {
        self.scale.eval(self.size)
    }
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            families: Rc::new(default_font_families()),
            variant: FontVariant {
                style: FontStyle::Normal,
                weight: FontWeight::REGULAR,
                stretch: FontStretch::Normal,
            },
            size: Length::pt(11.0),
            scale: Linear::ONE,
            strong: false,
            emph: false,
        }
    }
}

/// The default tree of font fallbacks.
fn default_font_families() -> FallbackTree {
    fallback! {
        list: ["sans-serif"],
        classes: {
            "serif"      => ["source serif pro", "noto serif"],
            "sans-serif" => ["source sans pro", "noto sans"],
            "monospace"  => ["source code pro", "noto sans mono"],
            "math"       => ["latin modern math", "serif"],
        },
        base: [
            "source sans pro",
            "noto sans",
            "segoe ui emoji",
            "noto emoji",
            "latin modern math",
        ],
    }
}

use std::rc::Rc;

use fontdock::{fallback, FallbackTree, FontStretch, FontStyle, FontVariant, FontWeight};

use super::Scope;
use crate::geom::{
    Align, ChildAlign, Dir, LayoutDirs, Length, Linear, Relative, Sides, Size,
};
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// The evaluation state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The scope that contains variable definitions.
    pub scope: Rc<Scope>,
    /// The current page state.
    pub page: StatePage,
    /// The current paragraph state.
    pub par: StatePar,
    /// The current font state.
    pub font: StateFont,
    /// The current directions.
    pub dirs: LayoutDirs,
    /// The current alignments.
    pub align: ChildAlign,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scope: Rc::new(crate::library::_std()),
            page: StatePage::default(),
            par: StatePar::default(),
            font: StateFont::default(),
            dirs: LayoutDirs::new(Dir::TTB, Dir::LTR),
            align: ChildAlign::new(Align::Start, Align::Start),
        }
    }
}

/// Defines page properties.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StatePage {
    /// The class of this page.
    pub class: PaperClass,
    /// The width and height of the page.
    pub size: Size,
    /// The amount of white space in the order [left, top, right, bottom]. If a
    /// side is set to `None`, the default for the paper class is used.
    pub margins: Sides<Option<Linear>>,
}

impl StatePage {
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

impl Default for StatePage {
    fn default() -> Self {
        Self::new(PAPER_A4)
    }
}

/// Defines paragraph properties.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StatePar {
    /// The spacing between words (dependent on scaled font size).
    pub word_spacing: Linear,
    /// The spacing between lines (dependent on scaled font size).
    pub line_spacing: Linear,
    /// The spacing between paragraphs (dependent on scaled font size).
    pub par_spacing: Linear,
}

impl Default for StatePar {
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
pub struct StateFont {
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

impl StateFont {
    /// The absolute font size.
    pub fn font_size(&self) -> Length {
        self.scale.resolve(self.size)
    }
}

impl Default for StateFont {
    fn default() -> Self {
        Self {
            /// The default tree of font fallbacks.
            families: Rc::new(fallback! {
                list: ["sans-serif"],
                classes: {
                    "serif"      => ["source serif pro", "noto serif"],
                    "sans-serif" => ["source sans pro", "noto sans"],
                    "monospace"  => ["source code pro", "noto sans mono"],
                },
                base: [
                    "source sans pro",
                    "noto sans",
                    "segoe ui emoji",
                    "noto emoji",
                    "latin modern math",
                ],
            }),
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

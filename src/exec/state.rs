use std::rc::Rc;

use fontdock::{fallback, FallbackTree, FontStretch, FontStyle, FontVariant, FontWeight};

use crate::geom::*;
use crate::layout::VerticalFontMetric;
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// The evaluation state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The current directions along which layouts are placed in their parents.
    pub dirs: LayoutDirs,
    /// The current alignments of layouts in their parents.
    pub aligns: LayoutAligns,
    /// The current page settings.
    pub page: PageState,
    /// The current paragraph settings.
    pub par: ParState,
    /// The current font settings.
    pub font: FontState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            dirs: LayoutDirs::new(Dir::TTB, Dir::LTR),
            aligns: LayoutAligns::new(Align::Start, Align::Start),
            page: PageState::default(),
            par: ParState::default(),
            font: FontState::default(),
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
    /// The amount of white space on each side of the page. If a side is set to
    /// `None`, the default for the paper class is used.
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
    /// The spacing between paragraphs (dependent on scaled font size).
    pub spacing: Linear,
    /// The spacing between lines (dependent on scaled font size).
    pub leading: Linear,
    /// The spacing between words (dependent on scaled font size).
    pub word_spacing: Linear,
}

impl Default for ParState {
    fn default() -> Self {
        Self {
            spacing: Relative::new(1.0).into(),
            leading: Relative::new(0.5).into(),
            word_spacing: Relative::new(0.25).into(),
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
    /// The top end of the text bounding box.
    pub top_edge: VerticalFontMetric,
    /// The bottom end of the text bounding box.
    pub bottom_edge: VerticalFontMetric,
    /// Whether the strong toggle is active or inactive. This determines
    /// whether the next `*` adds or removes font weight.
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. This determines
    /// whether the next `_` makes italic or non-italic.
    pub emph: bool,
}

impl FontState {
    /// Access the `families` mutably.
    pub fn families_mut(&mut self) -> &mut FallbackTree {
        Rc::make_mut(&mut self.families)
    }

    /// The absolute font size.
    pub fn font_size(&self) -> Length {
        self.scale.resolve(self.size)
    }
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            // The default tree of font fallbacks.
            families: Rc::new(fallback! {
                list: [],
                classes: { "monospace"  => ["inconsolata"] },
                base: ["eb garamond", "twitter color emoji"],
            }),
            variant: FontVariant {
                style: FontStyle::Normal,
                weight: FontWeight::REGULAR,
                stretch: FontStretch::Normal,
            },
            size: Length::pt(11.0),
            top_edge: VerticalFontMetric::CapHeight,
            bottom_edge: VerticalFontMetric::Baseline,
            scale: Linear::ONE,
            strong: false,
            emph: false,
        }
    }
}

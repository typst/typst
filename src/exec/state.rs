use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontVariant, FontWeight};

use crate::color::{Color, RgbaColor};
use crate::font::VerticalFontMetric;
use crate::geom::*;
use crate::layout::Fill;
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
    /// A list of font families with generic class definitions.
    pub families: Rc<FamilyList>,
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
    /// The glyph fill color / texture.
    pub color: Fill,
    /// Whether the strong toggle is active or inactive. This determines
    /// whether the next `*` adds or removes font weight.
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. This determines
    /// whether the next `_` makes italic or non-italic.
    pub emph: bool,
}

impl FontState {
    /// The resolved font size.
    pub fn resolve_size(&self) -> Length {
        self.scale.resolve(self.size)
    }

    /// Resolve font properties.
    pub fn resolve_props(&self) -> FontProps {
        let mut variant = self.variant;

        if self.strong {
            variant.weight = variant.weight.thicken(300);
        }

        if self.emph {
            variant.style = match variant.style {
                FontStyle::Normal => FontStyle::Italic,
                FontStyle::Italic => FontStyle::Normal,
                FontStyle::Oblique => FontStyle::Normal,
            }
        }

        FontProps {
            families: Rc::clone(&self.families),
            variant,
            size: self.resolve_size(),
            top_edge: self.top_edge,
            bottom_edge: self.bottom_edge,
            color: self.color,
        }
    }

    /// Access the `families` mutably.
    pub fn families_mut(&mut self) -> &mut FamilyList {
        Rc::make_mut(&mut self.families)
    }
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            families: Rc::new(FamilyList::default()),
            variant: FontVariant {
                style: FontStyle::Normal,
                weight: FontWeight::REGULAR,
                stretch: FontStretch::Normal,
            },
            size: Length::pt(11.0),
            top_edge: VerticalFontMetric::CapHeight,
            bottom_edge: VerticalFontMetric::Baseline,
            scale: Linear::ONE,
            color: Fill::Color(Color::Rgba(RgbaColor::BLACK)),
            strong: false,
            emph: false,
        }
    }
}

/// Properties used for font selection and layout.
#[derive(Debug, Clone, PartialEq)]
pub struct FontProps {
    /// The list of font families to use for shaping.
    pub families: Rc<FamilyList>,
    /// Which variant of the font to use.
    pub variant: FontVariant,
    /// The font size.
    pub size: Length,
    /// What line to consider the top edge of text.
    pub top_edge: VerticalFontMetric,
    /// What line to consider the bottom edge of text.
    pub bottom_edge: VerticalFontMetric,
    /// The color of the text.
    pub color: Fill,
}

/// Font family definitions.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct FamilyList {
    /// The user-defined list of font families.
    pub list: Vec<FontFamily>,
    /// Definition of serif font families.
    pub serif: Vec<String>,
    /// Definition of sans-serif font families.
    pub sans_serif: Vec<String>,
    /// Definition of monospace font families used for raw text.
    pub monospace: Vec<String>,
    /// Base fonts that are tried if the list has no match.
    pub base: Vec<String>,
}

impl FamilyList {
    /// Flat iterator over this map's family names.
    pub fn iter(&self) -> impl Iterator<Item = &str> + Clone {
        self.list
            .iter()
            .flat_map(move |family: &FontFamily| {
                match family {
                    FontFamily::Named(name) => std::slice::from_ref(name),
                    FontFamily::Serif => &self.serif,
                    FontFamily::SansSerif => &self.sans_serif,
                    FontFamily::Monospace => &self.monospace,
                }
            })
            .chain(&self.base)
            .map(String::as_str)
    }
}

impl Default for FamilyList {
    fn default() -> Self {
        Self {
            list: vec![FontFamily::Serif],
            serif: vec!["eb garamond".into()],
            sans_serif: vec![/* TODO */],
            monospace: vec!["inconsolata".into()],
            base: vec!["twitter color emoji".into()],
        }
    }
}

/// A generic or named font family.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FontFamily {
    Serif,
    SansSerif,
    Monospace,
    Named(String),
}

impl Display for FontFamily {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Serif => "serif",
            Self::SansSerif => "sans-serif",
            Self::Monospace => "monospace",
            Self::Named(s) => s,
        })
    }
}

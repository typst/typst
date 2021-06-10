use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::color::{Color, RgbaColor};
use crate::font::{FontStretch, FontStyle, FontVariant, FontWeight, VerticalFontMetric};
use crate::geom::*;
use crate::layout::Fill;
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// The execution state.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The current language-related settings.
    pub lang: LangState,
    /// The current page settings.
    pub page: PageState,
    /// The current paragraph settings.
    pub par: ParState,
    /// The current font settings.
    pub font: FontState,
    /// The current alignments of layouts in their parents.
    pub aligns: Gen<Align>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            lang: LangState::default(),
            page: PageState::default(),
            par: ParState::default(),
            font: FontState::default(),
            aligns: Gen::splat(Align::Start),
        }
    }
}

/// Defines language properties.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct LangState {
    /// The direction for text and other inline objects.
    pub dir: Dir,
}

impl Default for LangState {
    fn default() -> Self {
        Self { dir: Dir::LTR }
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
            margins: Sides::splat(None),
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
    // TODO: Don't ignore this.
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
    pub fill: Fill,
    /// Whether the strong toggle is active or inactive. This determines
    /// whether the next `*` adds or removes font weight.
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. This determines
    /// whether the next `_` makes italic or non-italic.
    pub emph: bool,
    /// The specifications for a strikethrough line, if any.
    pub strikethrough: Option<LineState>,
    /// The specifications for a underline, if any.
    pub underline: Option<LineState>,
    /// The specifications for a overline line, if any.
    pub overline: Option<LineState>,
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

        let size = self.resolve_size();
        FontProps {
            families: Rc::clone(&self.families),
            variant,
            size,
            top_edge: self.top_edge,
            bottom_edge: self.bottom_edge,
            strikethrough: self.strikethrough.map(|s| s.resolve_props(size, &self.fill)),
            underline: self.underline.map(|s| s.resolve_props(size, &self.fill)),
            overline: self.overline.map(|s| s.resolve_props(size, &self.fill)),
            fill: self.fill,
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
                stretch: FontStretch::NORMAL,
            },
            size: Length::pt(11.0),
            top_edge: VerticalFontMetric::CapHeight,
            bottom_edge: VerticalFontMetric::Baseline,
            scale: Linear::one(),
            fill: Fill::Color(Color::Rgba(RgbaColor::BLACK)),
            strong: false,
            emph: false,
            strikethrough: None,
            underline: None,
            overline: None,
        }
    }
}

/// Describes a line that could be positioned over or under text.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct LineState {
    /// Color of the line. Will default to text color if `None`.
    pub fill: Option<Fill>,
    /// Thickness of the line's stroke. Calling functions should attempt to
    /// read this value from the appropriate font tables if this is `None`.
    pub strength: Option<Linear>,
    /// Position of the line relative to the baseline. Calling functions should
    /// attempt to read this value from the appropriate font tables if this is
    /// `None`.
    pub position: Option<Linear>,
    /// Amount that the line will be longer or shorter than its associated text.
    pub extent: Linear,
}

impl LineState {
    pub fn resolve_props(&self, font_size: Length, fill: &Fill) -> LineProps {
        LineProps {
            fill: self.fill.unwrap_or_else(|| fill.clone()),
            strength: self.strength.map(|s| s.resolve(font_size)),
            position: self.position.map(|p| p.resolve(font_size)),
            extent: self.extent.resolve(font_size),
        }
    }
}

/// Properties used for font selection and layout.
#[derive(Debug, Clone, PartialEq, Hash)]
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
    /// The fill color of the text.
    pub fill: Fill,
    /// The specifications for a strikethrough line, if any.
    pub strikethrough: Option<LineProps>,
    /// The specifications for a underline, if any.
    pub underline: Option<LineProps>,
    /// The specifications for a overline line, if any.
    pub overline: Option<LineProps>,
}

/// Font family definitions.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

/// Describes a line that could be positioned over or under text.
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct LineProps {
    /// Color of the line.
    pub fill: Fill,
    /// Thickness of the line's stroke. Calling functions should attempt to
    /// read this value from the appropriate font tables if this is `None`.
    pub strength: Option<Length>,
    /// Position of the line relative to the baseline. Calling functions should
    /// attempt to read this value from the appropriate font tables if this is
    /// `None`.
    pub position: Option<Length>,
    /// Amount that the line will be longer or shorter than its associated text.
    pub extent: Length,
}

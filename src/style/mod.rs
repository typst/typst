//! Style properties.

mod paper;

pub use paper::*;

use std::fmt::{self, Debug, Formatter};
use std::rc::Rc;

use ttf_parser::Tag;

use crate::eval::Smart;
use crate::font::*;
use crate::geom::*;
use crate::util::EcoString;

/// Defines a set of properties a template can be instantiated with.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Style {
    /// The page settings.
    pub page: Rc<PageStyle>,
    /// The paragraph settings.
    pub par: Rc<ParStyle>,
    /// The current text settings.
    pub text: Rc<TextStyle>,
}

impl Style {
    /// Access the `page` style mutably.
    pub fn page_mut(&mut self) -> &mut PageStyle {
        Rc::make_mut(&mut self.page)
    }

    /// Access the `par` style mutably.
    pub fn par_mut(&mut self) -> &mut ParStyle {
        Rc::make_mut(&mut self.par)
    }

    /// Access the `text` style mutably.
    pub fn text_mut(&mut self) -> &mut TextStyle {
        Rc::make_mut(&mut self.text)
    }

    /// The resolved line spacing.
    pub fn leading(&self) -> Length {
        self.par.leading.resolve(self.text.size)
    }

    /// The resolved paragraph spacing.
    pub fn par_spacing(&self) -> Length {
        self.par.spacing.resolve(self.text.size)
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            page: Rc::new(PageStyle::default()),
            par: Rc::new(ParStyle::default()),
            text: Rc::new(TextStyle::default()),
        }
    }
}

/// Defines style properties of pages.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PageStyle {
    /// The class of this page.
    pub class: PaperClass,
    /// The width and height of the page.
    pub size: Size,
    /// The amount of white space on each side of the page. If a side is set to
    /// `None`, the default for the paper class is used.
    pub margins: Sides<Smart<Linear>>,
    /// The background fill of the page.
    pub fill: Option<Paint>,
}

impl PageStyle {
    /// The resolved margins.
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

impl Default for PageStyle {
    fn default() -> Self {
        let paper = Paper::A4;
        Self {
            class: paper.class(),
            size: paper.size(),
            margins: Sides::splat(Smart::Auto),
            fill: None,
        }
    }
}

/// Defines style properties of paragraphs.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ParStyle {
    /// The direction for text and inline objects.
    pub dir: Dir,
    /// How to align text and inline objects in their line.
    pub align: Align,
    /// The spacing between lines (dependent on scaled font size).
    pub leading: Linear,
    /// The spacing between paragraphs (dependent on scaled font size).
    pub spacing: Linear,
}

impl Default for ParStyle {
    fn default() -> Self {
        Self {
            dir: Dir::LTR,
            align: Align::Left,
            leading: Relative::new(0.65).into(),
            spacing: Relative::new(1.2).into(),
        }
    }
}

/// Defines style properties of text.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TextStyle {
    /// The font size.
    pub size: Length,
    /// The selected font variant (the final variant also depends on `strong`
    /// and `emph`).
    pub variant: FontVariant,
    /// The top end of the text bounding box.
    pub top_edge: VerticalFontMetric,
    /// The bottom end of the text bounding box.
    pub bottom_edge: VerticalFontMetric,
    /// Glyph color.
    pub fill: Paint,
    /// A list of font families with generic class definitions (the final
    /// family list also depends on `monospace`).
    pub families: Rc<FamilyStyle>,
    /// OpenType features.
    pub features: Rc<FontFeatures>,
    /// The amount of space that should be added between character.
    pub tracking: Em,
    /// Whether 300 extra font weight should be added to what is defined by the
    /// `variant`.
    pub strong: bool,
    /// Whether the the font style defined by the `variant` should be inverted.
    pub emph: bool,
    /// Whether a monospace font should be preferred.
    pub monospace: bool,
    /// Whether font fallback to a base list should occur.
    pub fallback: bool,
}

impl TextStyle {
    /// The resolved variant with `strong` and `emph` factored in.
    pub fn variant(&self) -> FontVariant {
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

        variant
    }

    /// The resolved family iterator.
    pub fn families(&self) -> impl Iterator<Item = &str> + Clone {
        let head = if self.monospace {
            self.families.monospace.as_slice()
        } else {
            &[]
        };

        let core = self.families.list.iter().flat_map(move |family| {
            match family {
                FontFamily::Named(name) => std::slice::from_ref(name),
                FontFamily::Serif => &self.families.serif,
                FontFamily::SansSerif => &self.families.sans_serif,
                FontFamily::Monospace => &self.families.monospace,
            }
        });

        let tail = if self.fallback {
            self.families.base.as_slice()
        } else {
            &[]
        };

        head.iter().chain(core).chain(tail).map(EcoString::as_str)
    }

    /// Access the `families` style mutably.
    pub fn families_mut(&mut self) -> &mut FamilyStyle {
        Rc::make_mut(&mut self.families)
    }

    /// Access the font `features` mutably.
    pub fn features_mut(&mut self) -> &mut FontFeatures {
        Rc::make_mut(&mut self.features)
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            size: Length::pt(11.0),
            variant: FontVariant {
                style: FontStyle::Normal,
                weight: FontWeight::REGULAR,
                stretch: FontStretch::NORMAL,
            },
            top_edge: VerticalFontMetric::CapHeight,
            bottom_edge: VerticalFontMetric::Baseline,
            fill: RgbaColor::BLACK.into(),
            families: Rc::new(FamilyStyle::default()),
            features: Rc::new(FontFeatures::default()),
            tracking: Em::zero(),
            strong: false,
            emph: false,
            monospace: false,
            fallback: true,
        }
    }
}

/// Font list with family definitions.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FamilyStyle {
    /// The user-defined list of font families.
    pub list: Vec<FontFamily>,
    /// Definition of serif font families.
    pub serif: Vec<EcoString>,
    /// Definition of sans-serif font families.
    pub sans_serif: Vec<EcoString>,
    /// Definition of monospace font families used for raw text.
    pub monospace: Vec<EcoString>,
    /// Base fonts that are tried as last resort.
    pub base: Vec<EcoString>,
}

impl Default for FamilyStyle {
    fn default() -> Self {
        Self {
            list: vec![FontFamily::SansSerif],
            serif: vec!["ibm plex serif".into()],
            sans_serif: vec!["ibm plex sans".into()],
            monospace: vec!["ibm plex mono".into()],
            base: vec![
                "ibm plex sans".into(),
                "latin modern math".into(),
                "twitter color emoji".into(),
            ],
        }
    }
}

/// A generic or named font family.
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum FontFamily {
    /// A family that has "serifs", small strokes attached to letters.
    Serif,
    /// A family in which glyphs do not have "serifs", small attached strokes.
    SansSerif,
    /// A family in which (almost) all glyphs are of equal width.
    Monospace,
    /// A specific family with a name.
    Named(EcoString),
}

impl Debug for FontFamily {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(match self {
            Self::Serif => "serif",
            Self::SansSerif => "sans-serif",
            Self::Monospace => "monospace",
            Self::Named(s) => s,
        })
    }
}

/// Whether various kinds of ligatures should appear.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontFeatures {
    /// Whether to apply kerning ("kern").
    pub kerning: bool,
    /// Whether the text should use small caps. ("smcp")
    pub smallcaps: bool,
    /// Whether to apply stylistic alternates. ("salt")
    pub alternates: bool,
    /// Which stylistic set to apply. ("ss01" - "ss20")
    pub stylistic_set: Option<StylisticSet>,
    /// Configuration of ligature features.
    pub ligatures: LigatureFeatures,
    /// Configuration of numbers features.
    pub numbers: NumberFeatures,
    /// Raw OpenType features to apply.
    pub raw: Vec<(Tag, u32)>,
}

impl Default for FontFeatures {
    fn default() -> Self {
        Self {
            kerning: true,
            smallcaps: false,
            alternates: false,
            stylistic_set: None,
            ligatures: LigatureFeatures::default(),
            numbers: NumberFeatures::default(),
            raw: vec![],
        }
    }
}

/// A stylistic set in a font face.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StylisticSet(u8);

impl StylisticSet {
    /// Creates a new set, clamping to 1-20.
    pub fn new(index: u8) -> Self {
        Self(index.clamp(1, 20))
    }

    /// Get the value, guaranteed to be 1-20.
    pub fn get(self) -> u8 {
        self.0
    }
}

/// Whether various kinds of ligatures should appear.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LigatureFeatures {
    /// Standard ligatures. ("liga", "clig")
    pub standard: bool,
    /// Ligatures that should be used sparringly. ("dlig")
    pub discretionary: bool,
    /// Historical ligatures. ("hlig")
    pub historical: bool,
}

impl Default for LigatureFeatures {
    fn default() -> Self {
        Self {
            standard: true,
            discretionary: false,
            historical: false,
        }
    }
}

/// Defines the style of numbers.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NumberFeatures {
    /// Whether to use lining or old-style numbers.
    pub type_: Smart<NumberType>,
    /// Whether to use proportional or tabular numbers.
    pub width: Smart<NumberWidth>,
    /// How to position numbers vertically.
    pub position: NumberPosition,
    /// Whether to have a slash through the zero glyph. ("zero")
    pub slashed_zero: bool,
    /// Whether to convert fractions. ("frac")
    pub fractions: bool,
}

impl Default for NumberFeatures {
    fn default() -> Self {
        Self {
            type_: Smart::Auto,
            width: Smart::Auto,
            position: NumberPosition::Normal,
            slashed_zero: false,
            fractions: false,
        }
    }
}

/// Which kind of numbers / figures to select.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberType {
    /// Numbers that fit well with capital text. ("lnum")
    Lining,
    /// Numbers that fit well into flow of upper- and lowercase text. ("onum")
    OldStyle,
}

/// The width of numbers / figures.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberWidth {
    /// Number widths are glyph specific. ("pnum")
    Proportional,
    /// All numbers are of equal width / monospaced. ("tnum")
    Tabular,
}

/// How to position numbers.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumberPosition {
    /// Numbers are positioned on the same baseline as text.
    Normal,
    /// Numbers are smaller and placed at the bottom. ("subs")
    Subscript,
    /// Numbers are smaller and placed at the top. ("sups")
    Superscript,
}

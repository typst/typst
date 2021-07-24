use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use crate::color::{Color, RgbaColor};
use crate::font::{FontStretch, FontStyle, FontVariant, FontWeight, VerticalFontMetric};
use crate::geom::*;
use crate::layout::Paint;
use crate::paper::{Paper, PaperClass, PAPER_A4};

/// The execution state.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct State {
    /// The current language-related settings.
    pub lang: LangState,
    /// The current page settings.
    pub page: PageState,
    /// The current paragraph settings.
    pub par: ParState,
    /// The current font settings.
    pub font: Rc<FontState>,
    /// The current alignments of layouts in their parents.
    pub aligns: Gen<Align>,
}

impl State {
    /// Access the `font` state mutably.
    pub fn font_mut(&mut self) -> &mut FontState {
        Rc::make_mut(&mut self.font)
    }
}

/// Defines language properties.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontState {
    /// A list of font families with generic class definitions.
    pub families: Rc<FamilyList>,
    /// The selected font variant.
    pub variant: FontVariant,
    /// Whether the strong toggle is active or inactive. This determines
    /// whether the next `*` adds or removes font weight.
    pub strong: bool,
    /// Whether the emphasis toggle is active or inactive. This determines
    /// whether the next `_` makes italic or non-italic.
    pub emph: bool,
    /// Whether the monospace toggle is active or inactive.
    pub monospace: bool,
    /// The font size.
    pub size: Length,
    /// The top end of the text bounding box.
    pub top_edge: VerticalFontMetric,
    /// The bottom end of the text bounding box.
    pub bottom_edge: VerticalFontMetric,
    /// Glyph color.
    pub fill: Paint,
    /// The specifications for a strikethrough line, if any.
    pub strikethrough: Option<Rc<LineState>>,
    /// The specifications for a underline, if any.
    pub underline: Option<Rc<LineState>>,
    /// The specifications for a overline line, if any.
    pub overline: Option<Rc<LineState>>,
}

impl FontState {
    /// Access the `families` mutably.
    pub fn families_mut(&mut self) -> &mut FamilyList {
        Rc::make_mut(&mut self.families)
    }

    /// The canonical family iterator.
    pub fn families(&self) -> impl Iterator<Item = &str> + Clone {
        let head = if self.monospace {
            self.families.monospace.as_slice()
        } else {
            &[]
        };
        head.iter().map(String::as_str).chain(self.families.iter())
    }

    /// The canonical variant with `strong` and `emph` factored in.
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
            strong: false,
            emph: false,
            monospace: false,
            size: Length::pt(11.0),
            top_edge: VerticalFontMetric::CapHeight,
            bottom_edge: VerticalFontMetric::Baseline,
            fill: Paint::Color(Color::Rgba(RgbaColor::BLACK)),
            strikethrough: None,
            underline: None,
            overline: None,
        }
    }
}

/// Describes a line that could be positioned over, under or on top of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LineState {
    /// Stroke color of the line.
    ///
    /// Defaults to the text color if `None`.
    pub stroke: Option<Paint>,
    /// Thickness of the line's stroke. Calling functions should attempt to
    /// read this value from the appropriate font tables if this is `None`.
    pub thickness: Option<Linear>,
    /// Position of the line relative to the baseline. Calling functions should
    /// attempt to read this value from the appropriate font tables if this is
    /// `None`.
    pub offset: Option<Linear>,
    /// Amount that the line will be longer or shorter than its associated text.
    pub extent: Linear,
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
            sans_serif: vec!["pt sans".into()],
            monospace: vec!["inconsolata".into()],
            base: vec!["twitter color emoji".into(), "latin modern math".into()],
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

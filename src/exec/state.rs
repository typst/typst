use std::rc::Rc;

use crate::color::{Color, RgbaColor};
use crate::font::{
    FontFamily, FontStretch, FontStyle, FontVariant, FontWeight, VerticalFontMetric,
};
use crate::geom::*;
use crate::layout::Paint;
use crate::paper::{PaperClass, PAPER_A4};

/// The execution state.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct State {
    /// The direction for text and other inline objects.
    pub dirs: Gen<Dir>,
    /// The current alignments of layouts in their parents.
    pub aligns: Gen<Align>,
    /// The current page settings.
    pub page: Rc<PageState>,
    /// The current paragraph settings.
    pub par: Rc<ParState>,
    /// The current font settings.
    pub font: Rc<FontState>,
}

impl State {
    /// Access the `page` state mutably.
    pub fn page_mut(&mut self) -> &mut PageState {
        Rc::make_mut(&mut self.page)
    }

    /// Access the `par` state mutably.
    pub fn par_mut(&mut self) -> &mut ParState {
        Rc::make_mut(&mut self.par)
    }

    /// Access the `font` state mutably.
    pub fn font_mut(&mut self) -> &mut FontState {
        Rc::make_mut(&mut self.font)
    }

    /// The resolved line spacing.
    pub fn line_spacing(&self) -> Length {
        self.par.line_spacing.resolve(self.font.size)
    }

    /// The resolved paragraph spacing.
    pub fn par_spacing(&self) -> Length {
        self.par.par_spacing.resolve(self.font.size)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            dirs: Gen::new(Dir::LTR, Dir::TTB),
            aligns: Gen::splat(Align::Start),
            page: Rc::new(PageState::default()),
            par: Rc::new(ParState::default()),
            font: Rc::new(FontState::default()),
        }
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

impl Default for PageState {
    fn default() -> Self {
        let paper = PAPER_A4;
        Self {
            class: paper.class,
            size: paper.size(),
            margins: Sides::splat(None),
        }
    }
}

/// Style paragraph properties.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ParState {
    /// The spacing between paragraphs (dependent on scaled font size).
    pub par_spacing: Linear,
    /// The spacing between lines (dependent on scaled font size).
    pub line_spacing: Linear,
}

impl Default for ParState {
    fn default() -> Self {
        Self {
            par_spacing: Relative::new(1.0).into(),
            line_spacing: Relative::new(0.5).into(),
        }
    }
}

/// Defines font properties.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontState {
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
    pub families: Rc<FamilyState>,
    /// The specifications for a strikethrough line, if any.
    pub strikethrough: Option<Rc<LineState>>,
    /// The specifications for a underline, if any.
    pub underline: Option<Rc<LineState>>,
    /// The specifications for a overline line, if any.
    pub overline: Option<Rc<LineState>>,
}

impl FontState {
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

        let core = self.families.list.iter().flat_map(move |family: &FontFamily| {
            match family {
                FontFamily::Named(name) => std::slice::from_ref(name),
                FontFamily::Serif => &self.families.serif,
                FontFamily::SansSerif => &self.families.sans_serif,
                FontFamily::Monospace => &self.families.monospace,
            }
        });

        head.iter()
            .chain(core)
            .chain(self.families.base.iter())
            .map(String::as_str)
    }

    /// Access the `families` state mutably.
    pub fn families_mut(&mut self) -> &mut FamilyState {
        Rc::make_mut(&mut self.families)
    }
}

impl Default for FontState {
    fn default() -> Self {
        Self {
            families: Rc::new(FamilyState::default()),
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

/// Font family definitions.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FamilyState {
    /// The user-defined list of font families.
    pub list: Rc<Vec<FontFamily>>,
    /// Definition of serif font families.
    pub serif: Rc<Vec<String>>,
    /// Definition of sans-serif font families.
    pub sans_serif: Rc<Vec<String>>,
    /// Definition of monospace font families used for raw text.
    pub monospace: Rc<Vec<String>>,
    /// Base fonts that are tried as last resort.
    pub base: Rc<Vec<String>>,
}

impl Default for FamilyState {
    fn default() -> Self {
        Self {
            list: Rc::new(vec![FontFamily::Serif]),
            serif: Rc::new(vec!["eb garamond".into()]),
            sans_serif: Rc::new(vec!["pt sans".into()]),
            monospace: Rc::new(vec!["inconsolata".into()]),
            base: Rc::new(vec![
                "twitter color emoji".into(),
                "latin modern math".into(),
            ]),
        }
    }
}

/// Defines a line that is positioned over, under or on top of text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct LineState {
    /// Stroke color of the line, defaults to the text color if `None`.
    pub stroke: Option<Paint>,
    /// Thickness of the line's strokes (dependent on scaled font size), read
    /// from the font tables if `None`.
    pub thickness: Option<Linear>,
    /// Position of the line relative to the baseline (dependent on scaled font
    /// size), read from the font tables if `None`.
    pub offset: Option<Linear>,
    /// Amount that the line will be longer or shorter than its associated text
    /// (dependent on scaled font size).
    pub extent: Linear,
}

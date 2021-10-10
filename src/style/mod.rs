//! Style properties.

mod paper;

pub use paper::*;

use std::rc::Rc;

use crate::font::{
    FontFamily, FontStretch, FontStyle, FontVariant, FontWeight, VerticalFontMetric,
};
use crate::geom::*;

/// Defines a set of properties a template can be instantiated with.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Style {
    /// The direction for text and other inline objects.
    pub dir: Dir,
    /// The alignments of layouts in their parents.
    pub aligns: Gen<Align>,
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
    pub fn line_spacing(&self) -> Length {
        self.par.line_spacing.resolve(self.text.size)
    }

    /// The resolved paragraph spacing.
    pub fn par_spacing(&self) -> Length {
        self.par.par_spacing.resolve(self.text.size)
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            dir: Dir::LTR,
            aligns: Gen::splat(Align::Start),
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
    pub margins: Sides<Option<Linear>>,
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
        let paper = Paper::ISO_A4;
        Self {
            class: paper.class(),
            size: paper.size(),
            margins: Sides::splat(None),
        }
    }
}

/// Defines style properties of paragraphs.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ParStyle {
    /// The spacing between paragraphs (dependent on scaled font size).
    pub par_spacing: Linear,
    /// The spacing between lines (dependent on scaled font size).
    pub line_spacing: Linear,
}

impl Default for ParStyle {
    fn default() -> Self {
        Self {
            par_spacing: Relative::new(1.2).into(),
            line_spacing: Relative::new(0.65).into(),
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

        head.iter().chain(core).chain(tail).map(String::as_str)
    }

    /// Access the `families` style mutably.
    pub fn families_mut(&mut self) -> &mut FamilyStyle {
        Rc::make_mut(&mut self.families)
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
            fill: Paint::Color(Color::Rgba(RgbaColor::BLACK)),
            families: Rc::new(FamilyStyle::default()),
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

impl Default for FamilyStyle {
    fn default() -> Self {
        Self {
            list: Rc::new(vec![FontFamily::SansSerif]),
            serif: Rc::new(vec!["ibm plex serif".into()]),
            sans_serif: Rc::new(vec!["ibm plex sans".into()]),
            monospace: Rc::new(vec!["ibm plex mono".into()]),
            base: Rc::new(vec![
                "ibm plex sans".into(),
                "latin modern math".into(),
                "twitter color emoji".into(),
            ]),
        }
    }
}

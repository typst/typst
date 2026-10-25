//! Font handling.

pub mod color;

mod book;
mod exceptions;
mod info;
mod metrics;
mod tag;
mod variant;
mod variations;

pub use self::book::FontBook;
pub use self::info::{Coverage, FontFlags, FontInfo};
pub use self::metrics::{
    FontMetrics, LineMetrics, MathConstants, ScriptMetrics, TextEdgeBounds,
    VerticalFontMetric,
};
pub use self::tag::Tag;
pub use self::variant::{FontStretch, FontStyle, FontVariant, FontWeight};
pub use self::variations::{AxisValue, FontAxis, FontVariations, StandardAxes};

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use read_fonts::types::GlyphId;

use self::exceptions::find_exception;
use self::info::find_name;
use crate::foundations::Bytes;
use crate::layout::{Abs, Em};
use crate::text::{BottomEdge, TopEdge};

/// An OpenType font.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone)]
pub struct Font(Arc<FontInner>);

/// The internal representation of a [`Font`].
struct FontInner {
    /// The font's index in the buffer.
    index: u32,
    /// Metadata about the font.
    info: FontInfo,
    /// The font at its default location.
    font: read_fonts::model::Font,
    /// The raw font data, possibly shared with other fonts from the same
    /// collection.
    data: Bytes,
}

impl Font {
    /// Parse a font from data and collection index.
    pub fn new(data: Bytes, index: u32) -> Option<Self> {
        let font = read_fonts::model::Font::new(
            read_fonts::model::Blob::Shared(Arc::new(data.clone())),
            index,
        )?;
        let info = FontInfo::from_font(&font)?;
        Some(Self(Arc::new(FontInner { index, info, font, data })))
    }

    /// Parse all fonts in the given data.
    pub fn iter(data: Bytes) -> impl Iterator<Item = Self> {
        let count = read_fonts::model::Format::new(&data).map_or(1, |f| f.num_fonts());
        (0..count).filter_map(move |index| Self::new(data.clone(), index))
    }

    /// The underlying buffer.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The font's index in the buffer.
    pub fn index(&self) -> u32 {
        self.0.index
    }

    /// The font's metadata.
    pub fn info(&self) -> &FontInfo {
        &self.0.info
    }

    /// Determine the font's PostScript name.
    pub fn post_script_name(&self) -> Option<String> {
        find_name(&self.0.font, read_fonts::types::NameId::POSTSCRIPT_NAME)
    }

    /// Instantiates the font with specific text properties. The resulting
    /// type allows access to methods that depend on coordinates.
    #[comemo::memoize]
    pub fn instantiate(
        self,
        variant: FontVariant,
        size: Abs,
        custom: &FontVariations,
    ) -> FontInstance {
        let axes = &self.info().axes;
        let automatic = FontVariations::resolve(axes, variant, size);
        let full = automatic.chain(custom).normalized();
        self.instantiate_impl(full)
    }

    /// Instantiates the font with specific variation coordinates. The resulting
    /// type allows access to methods that depend on coordinates.
    #[comemo::memoize]
    fn instantiate_impl(self, variations: FontVariations) -> FontInstance {
        let instance =
            self.0
                .font
                .instance_builder()
                .variations(
                    variations.0.iter().map(|&(tag, value)| {
                        (read_fonts::types::Tag::from(tag), value.0)
                    }),
                )
                .build();
        let metrics = FontMetrics::from_font(&instance);
        FontInstance(Arc::new(FontInstanceInner {
            metrics,
            instance,
            variations,
            font: self,
        }))
    }
}

impl Debug for Font {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Font({}, {:?})", self.info().family, self.info().variant)
    }
}

impl Hash for Font {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.data.hash(state);
        self.0.index.hash(state);
    }
}

impl Eq for Font {}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        self.0.data == other.0.data && self.0.index == other.0.index
    }
}

/// An OpenType font with fixed variation coordinates.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone)]
pub struct FontInstance(Arc<FontInstanceInner>);

/// The internal representation of a [`FontInstance`].
struct FontInstanceInner {
    /// The font's metrics.
    metrics: FontMetrics,
    /// The font at the given instance.
    instance: read_fonts::model::Font,
    /// The instance's variation coordinates.
    variations: FontVariations,
    /// The underlying font.
    font: Font,
}

impl FontInstance {
    /// The instance's underlying font.
    pub fn font(&self) -> &Font {
        &self.0.font
    }

    /// The instance's variation coordinates.
    pub fn variations(&self) -> &FontVariations {
        &self.0.variations
    }

    /// The font's metrics.
    pub fn metrics(&self) -> &FontMetrics {
        &self.0.metrics
    }

    /// The font's math constants.
    #[inline]
    pub fn math(&self) -> &MathConstants {
        self.0.metrics.math.get_or_init(|| MathConstants::new(self))
    }

    /// The number of font units per one em.
    pub fn units_per_em(&self) -> f64 {
        self.0.metrics.units_per_em
    }

    /// Convert from font units to an em length.
    pub fn to_em(&self, units: impl Into<f64>) -> Em {
        Em::from_units(units, self.units_per_em())
    }

    /// Look up the horizontal advance width of a glyph.
    pub fn x_advance(&self, glyph: u32) -> Em {
        self.to_em(
            self.0
                .instance
                .glyph_metrics()
                .h_advance_exact(GlyphId::new(glyph))
                .to_f64(),
        )
    }

    /// Look up the vertical advance width of a glyph.
    pub fn y_advance(&self, glyph: u32) -> Em {
        self.to_em(
            self.0
                .instance
                .glyph_metrics()
                .v_advance_exact(GlyphId::new(glyph))
                .to_f64(),
        )
    }

    /// A reference to the font at the given instance.
    pub fn instance(&self) -> &read_fonts::model::Font {
        &self.0.instance
    }

    /// Resolve the top and bottom edges of text.
    pub fn edges(
        &self,
        top_edge: TopEdge,
        bottom_edge: BottomEdge,
        font_size: Abs,
        bounds: TextEdgeBounds,
    ) -> (Abs, Abs) {
        let cell = OnceCell::new();
        let extents = |gid,
                       f: fn(
            read_fonts::model::metrics::GlyphExtents<read_fonts::types::F48Dot16>,
        ) -> read_fonts::types::F48Dot16| {
            cell.get_or_init(|| {
                self.instance()
                    .glyph_metrics()
                    .extents_exact(read_fonts::types::GlyphId::new(gid))
            })
            .map(|extents| self.to_em(f(extents).to_f64()).at(font_size))
            .unwrap_or_default()
        };

        let top = match top_edge {
            TopEdge::Metric(metric) => match metric.try_into() {
                Ok(metric) => self.metrics().vertical(metric).at(font_size),
                Err(()) => match bounds {
                    TextEdgeBounds::Zero => Abs::zero(),
                    TextEdgeBounds::Frame(frame) => frame.ascent(),
                    TextEdgeBounds::Glyph(gid) => extents(gid, |e| e.y_bearing),
                },
            },
            TopEdge::Length(length) => length.at(font_size),
        };

        let bottom = match bottom_edge {
            BottomEdge::Metric(metric) => match metric.try_into() {
                Ok(metric) => -self.metrics().vertical(metric).at(font_size),
                Err(()) => match bounds {
                    TextEdgeBounds::Zero => Abs::zero(),
                    TextEdgeBounds::Frame(frame) => frame.descent(),
                    TextEdgeBounds::Glyph(gid) => {
                        -extents(gid, |e| e.y_bearing - e.height)
                    }
                },
            },
            BottomEdge::Length(length) => -length.at(font_size),
        };

        (top, bottom)
    }
}

impl Deref for FontInstance {
    type Target = Font;

    fn deref(&self) -> &Self::Target {
        self.font()
    }
}

impl Debug for FontInstance {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("FontInstance")
            .field("font", self.font())
            .field("variations", self.variations())
            .finish()
    }
}

impl Hash for FontInstance {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.font.hash(state);
        self.0.variations.hash(state);
    }
}

impl Eq for FontInstance {}

impl PartialEq for FontInstance {
    fn eq(&self, other: &Self) -> bool {
        self.0.font == other.0.font && self.0.variations == other.0.variations
    }
}

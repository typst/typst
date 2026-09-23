//! Font handling.

pub mod color;

mod book;
mod exceptions;
mod info;
mod metrics;
mod tag;
mod variant;
mod variations;

use skrifa::outline::pen::PathStyle;
use skrifa::outline::{AdjustedMetrics, DrawSettings, OutlinePen};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};

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
use std::sync::{Arc, OnceLock};

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
    /// Cached shaper data for the font.
    harfrust: harfrust::ShaperData,
    // NOTE: `skrifa` references `data`, so it's important for `data` to be
    // dropped after `skrifa` or `skrifa` will be left dangling while the data
    // is dropped. Fields are dropped in declaration order, so `data` needs to
    // be declared after `skrifa`.
    /// The underlying skrifa face.
    skrifa: skrifa::FontRef<'static>,
    /// The raw font data, possibly shared with other fonts from the same
    /// collection. The vector's allocation must not move, because `skrifa`
    /// points into it using unsafe code.
    data: Bytes,
}

impl Font {
    /// Parse a font from data and collection index.
    pub fn new(data: Bytes, index: u32) -> Option<Self> {
        // Safety:
        // - The slices's location is stable in memory:
        //   - We don't move the underlying vector
        //   - Nobody else can move it since we have a strong ref to the `Arc`.
        // - The internal 'static lifetime is not leaked because its rewritten
        //   to the self-lifetime in `ttf()`. TODO: this comment.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };

        let skrifa = skrifa::FontRef::from_index(slice, index).ok()?;
        let harfrust = harfrust::ShaperData::new(&skrifa);
        let info = FontInfo::from_skrifa(&skrifa)?;

        Some(Self(Arc::new(FontInner { index, info, harfrust, skrifa, data })))
    }

    /// Parse all fonts in the given data.
    pub fn iter(data: Bytes) -> impl Iterator<Item = Self> {
        let count = match skrifa::raw::FileRef::new(&data) {
            Ok(skrifa::raw::FileRef::Font(_)) => 1,
            Ok(skrifa::raw::FileRef::Collection(ttc)) => ttc.len(),
            _ => 0,
        };
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

    /// The underlying skrifa face.
    pub fn skrifa(&self) -> &skrifa::FontRef<'_> {
        &self.0.skrifa
    }

    /// Determine the font's PostScript name.
    pub fn post_script_name(&self) -> Option<String> {
        find_name(&self.0.skrifa, skrifa::string::StringId::POSTSCRIPT_NAME)
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
        let instance = harfrust::ShaperInstance::from_variations(
            &self.0.skrifa,
            variations.0.iter().map(|&(tag, value)| harfrust::Variation {
                tag: tag.into(),
                value: value.0,
            }),
        );

        let metrics = FontMetrics::from_skrifa(
            &self.0.skrifa,
            skrifa::instance::LocationRef::new(instance.coords()),
        );

        FontInstance(Arc::new(FontInstanceInner {
            metrics,
            shaper: OnceLock::new(),
            glyph_metrics: OnceLock::new(),
            charmap: OnceLock::new(),
            outlines: OnceLock::new(),
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
    /// The instance's variation coordinates.
    variations: FontVariations,
    // NOTE: `shaper` and `glyph_metrics` reference `instance` and `font`, so
    // it's important that they are dropped after them or they will be left
    // dangling while they're dropped. Fields are dropped in declaration order,
    // so `instance` and `data` need to be declared after `shaper` and
    // `glyph_metrics`.
    /// The shaper for this instance.
    shaper: OnceLock<harfrust::Shaper<'static>>,
    /// Glyph metrics for this instance in font units.
    glyph_metrics: OnceLock<skrifa::metrics::GlyphMetrics<'static>>,
    ///
    charmap: OnceLock<skrifa::charmap::Charmap<'static>>,
    ///
    outlines: OnceLock<skrifa::outline::OutlineGlyphCollection<'static>>,
    // TODO: this comment.
    /// The instance's normalized variation coordinates.
    instance: harfrust::ShaperInstance,
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
    pub fn x_advance(&self, gid: impl Into<GlyphId>) -> Option<Em> {
        self.glyph_metrics()
            .advance_width(gid.into())
            .map(|units| self.to_em(units))
    }

    ///
    pub fn outline_glyph(
        &self,
        gid: impl Into<GlyphId>,
        pen: &mut impl OutlinePen,
    ) -> Option<AdjustedMetrics> {
        let settings = DrawSettings::unhinted(Size::unscaled(), self.location())
            .with_path_style(PathStyle::HarfBuzz);
        self.outlines().get(gid.into())?.draw(settings, pen).ok()
    }

    /// Look up the vertical advance width of a glyph.
    pub fn y_advance(&self, gid: impl Into<GlyphId>) -> Option<Em> {
        // TODO manually: skrifa doesn't provide these: https://github.com/googlefonts/fontations/pull/1552
        // self.0
        //     .rusty
        //     .glyph_ver_advance(GlyphId(glyph))
        //     .map(|units| self.to_em(units))
        None
    }

    ///
    pub fn bounding_box(
        &self,
        gid: impl Into<GlyphId>,
    ) -> Option<skrifa::metrics::BoundingBox> {
        self.glyph_metrics().bounds(gid.into())
    }

    ///
    pub fn glyph_index(&self, c: char) -> Option<skrifa::GlyphId> {
        self.charmap().map(c)
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
        let bbox = |gid, f: fn(skrifa::metrics::BoundingBox) -> f32| {
            cell.get_or_init(|| self.bounding_box(gid))
                .map(|bbox| self.to_em(f(bbox)).at(font_size))
                .unwrap_or_default()
        };

        let top = match top_edge {
            TopEdge::Metric(metric) => match metric.try_into() {
                Ok(metric) => self.metrics().vertical(metric).at(font_size),
                Err(()) => match bounds {
                    TextEdgeBounds::Zero => Abs::zero(),
                    TextEdgeBounds::Frame(frame) => frame.ascent(),
                    TextEdgeBounds::Glyph(gid) => bbox(gid, |b| b.y_max),
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
                    TextEdgeBounds::Glyph(gid) => -bbox(gid, |b| b.y_min),
                },
            },
            BottomEdge::Length(length) => -length.at(font_size),
        };

        (top, bottom)
    }

    /// The shaper for this instance.
    pub fn shaper(&self) -> &harfrust::Shaper<'_> {
        self.0.shaper.get_or_init(|| {
            let (font, instance) = self.borrow_parts();
            font.0
                .harfrust
                .shaper(&font.0.skrifa)
                .instance(Some(instance))
                .build()
        })
    }

    /// Glyph metrics for this instance in font units.
    fn glyph_metrics(&self) -> &skrifa::metrics::GlyphMetrics<'_> {
        self.0.glyph_metrics.get_or_init(|| {
            let (font, instance) = self.borrow_parts();
            skrifa::metrics::GlyphMetrics::new(
                &font.0.skrifa,
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::new(instance.coords()),
            )
        })
    }

    fn charmap(&self) -> &skrifa::charmap::Charmap<'_> {
        self.0.charmap.get_or_init(|| {
            let (font, _) = self.borrow_parts();
            font.0.skrifa.charmap()
        })
    }

    fn outlines(&self) -> &skrifa::outline::OutlineGlyphCollection<'_> {
        self.0.outlines.get_or_init(|| {
            let (font, _) = self.borrow_parts();
            font.0.skrifa.outline_glyphs()
        })
    }

    /// The instance's normalized variation coordinates.
    pub fn location(&self) -> skrifa::instance::LocationRef<'_> {
        skrifa::instance::LocationRef::new(self.0.instance.coords())
    }

    // TODO: this comment.
    /// Reinterprets the font and shaper instance as `'static`.
    ///
    /// Safety:
    /// - Both live in the `Arc` this is called through, so their addresses are
    ///   already fixed and they stay alive as long as it does.
    /// - The `'static` lifetime is not leaked: it is only used to build values
    ///   stored in the same `Arc`, and those are handed out rewritten to the
    ///   self-lifetime by `shaper()` and `glyph_metrics()`.
    /// - Those values are declared before `instance` and `font`, so they are
    ///   dropped first.
    fn borrow_parts(&self) -> (&'static Font, &'static harfrust::ShaperInstance) {
        let ptr = Arc::as_ptr(&self.0);
        unsafe { (&*(&raw const (*ptr).font), &*(&raw const (*ptr).instance)) }
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

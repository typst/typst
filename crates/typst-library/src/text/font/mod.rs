//! Font handling.

pub mod color;

mod book;
mod exceptions;
mod variant;

pub use self::book::{Coverage, FontBook, FontFlags, FontInfo};
pub use self::variant::{FontStretch, FontStyle, FontVariant, FontWeight};

use std::cell::OnceCell;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use ttf_parser::{GlyphId, name_id};

use self::book::find_name;
use crate::foundations::{Bytes, Cast};
use crate::layout::{Abs, Em, Frame};
use crate::text::{
    BottomEdge, DEFAULT_SUBSCRIPT_METRICS, DEFAULT_SUPERSCRIPT_METRICS, TopEdge,
};

/// An OpenType font.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone)]
pub struct Font(Arc<Repr>);

/// The internal representation of a font.
struct Repr {
    /// The font's index in the buffer.
    index: u32,
    /// Metadata about the font.
    info: FontInfo,
    /// The font's metrics.
    metrics: FontMetrics,
    /// The underlying ttf-parser face.
    ttf: ttf_parser::Face<'static>,
    /// The underlying rustybuzz face.
    rusty: rustybuzz::Face<'static>,
    // NOTE: `ttf` and `rusty` reference `data`, so it's important for `data`
    // to be dropped after them or they will be left dangling while they're
    // dropped. Fields are dropped in declaration order, so `data` needs to be
    // declared after `ttf` and `rusty`.
    /// The raw font data, possibly shared with other fonts from the same
    /// collection. The vector's allocation must not move, because `ttf` points
    /// into it using unsafe code.
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
        //   to the self-lifetime in `ttf()`.
        let slice: &'static [u8] =
            unsafe { std::slice::from_raw_parts(data.as_ptr(), data.len()) };

        let ttf = ttf_parser::Face::parse(slice, index).ok()?;
        let rusty = rustybuzz::Face::from_slice(slice, index)?;
        let metrics = FontMetrics::from_ttf(&ttf);
        let info = FontInfo::from_ttf(&ttf)?;

        Some(Self(Arc::new(Repr { data, index, info, metrics, ttf, rusty })))
    }

    /// Parse all fonts in the given data.
    pub fn iter(data: Bytes) -> impl Iterator<Item = Self> {
        let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
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

    /// The font's metrics.
    pub fn metrics(&self) -> &FontMetrics {
        &self.0.metrics
    }

    /// The font's math constants.
    #[inline]
    pub fn math(&self) -> &MathConstants {
        self.0.metrics.math.get_or_init(|| FontMetrics::init_math(self))
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
    pub fn x_advance(&self, glyph: u16) -> Option<Em> {
        self.0
            .ttf
            .glyph_hor_advance(GlyphId(glyph))
            .map(|units| self.to_em(units))
    }

    /// Look up the vertical advance width of a glyph.
    pub fn y_advance(&self, glyph: u16) -> Option<Em> {
        self.0
            .ttf
            .glyph_ver_advance(GlyphId(glyph))
            .map(|units| self.to_em(units))
    }

    /// Lookup a name by id.
    pub fn find_name(&self, id: u16) -> Option<String> {
        find_name(&self.0.ttf, id)
    }

    /// A reference to the underlying `ttf-parser` face.
    pub fn ttf(&self) -> &ttf_parser::Face<'_> {
        // We can't implement Deref because that would leak the
        // internal 'static lifetime.
        &self.0.ttf
    }

    /// A reference to the underlying `rustybuzz` face.
    pub fn rusty(&self) -> &rustybuzz::Face<'_> {
        // We can't implement Deref because that would leak the
        // internal 'static lifetime.
        &self.0.rusty
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
        let bbox = |gid, f: fn(ttf_parser::Rect) -> i16| {
            cell.get_or_init(|| self.ttf().glyph_bounding_box(GlyphId(gid)))
                .map(|bbox| self.to_em(f(bbox)).at(font_size))
                .unwrap_or_default()
        };

        let top = match top_edge {
            TopEdge::Metric(metric) => match metric.try_into() {
                Ok(metric) => self.metrics().vertical(metric).at(font_size),
                Err(_) => match bounds {
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
                Err(_) => match bounds {
                    TextEdgeBounds::Zero => Abs::zero(),
                    TextEdgeBounds::Frame(frame) => frame.descent(),
                    TextEdgeBounds::Glyph(gid) => -bbox(gid, |b| b.y_min),
                },
            },
            BottomEdge::Length(length) => -length.at(font_size),
        };

        (top, bottom)
    }
}

impl Hash for Font {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.data.hash(state);
        self.0.index.hash(state);
    }
}

impl Debug for Font {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Font({}, {:?})", self.info().family, self.info().variant)
    }
}

impl Eq for Font {}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        self.0.data == other.0.data && self.0.index == other.0.index
    }
}

/// Metrics of a font.
#[derive(Debug, Clone)]
pub struct FontMetrics {
    /// How many font units represent one em unit.
    pub units_per_em: f64,
    /// The distance from the baseline to the typographic ascender.
    pub ascender: Em,
    /// The approximate height of uppercase letters.
    pub cap_height: Em,
    /// The approximate height of non-ascending lowercase letters.
    pub x_height: Em,
    /// The distance from the baseline to the typographic descender.
    pub descender: Em,
    /// Recommended metrics for a strikethrough line.
    pub strikethrough: LineMetrics,
    /// Recommended metrics for an underline.
    pub underline: LineMetrics,
    /// Recommended metrics for an overline.
    pub overline: LineMetrics,
    /// Metrics for subscripts, if provided by the font.
    pub subscript: Option<ScriptMetrics>,
    /// Metrics for superscripts, if provided by the font.
    pub superscript: Option<ScriptMetrics>,
    /// Metrics for math layout.
    pub math: OnceLock<Box<MathConstants>>,
}

impl FontMetrics {
    /// Extract the font's metrics.
    pub fn from_ttf(ttf: &ttf_parser::Face) -> Self {
        let units_per_em = f64::from(ttf.units_per_em());
        let to_em = |units| Em::from_units(units, units_per_em);

        let ascender = to_em(ttf.typographic_ascender().unwrap_or(ttf.ascender()));
        let cap_height = ttf.capital_height().filter(|&h| h > 0).map_or(ascender, to_em);
        let x_height = ttf.x_height().filter(|&h| h > 0).map_or(ascender, to_em);
        let descender = to_em(ttf.typographic_descender().unwrap_or(ttf.descender()));

        let strikeout = ttf.strikeout_metrics();
        let underline = ttf.underline_metrics();

        let strikethrough = LineMetrics {
            position: strikeout.map_or(Em::new(0.25), |s| to_em(s.position)),
            thickness: strikeout
                .or(underline)
                .map_or(Em::new(0.06), |s| to_em(s.thickness)),
        };

        let underline = LineMetrics {
            position: underline.map_or(Em::new(-0.2), |s| to_em(s.position)),
            thickness: underline
                .or(strikeout)
                .map_or(Em::new(0.06), |s| to_em(s.thickness)),
        };

        let overline = LineMetrics {
            position: cap_height + Em::new(0.1),
            thickness: underline.thickness,
        };

        let subscript = ttf.subscript_metrics().map(|metrics| ScriptMetrics {
            width: to_em(metrics.x_size),
            height: to_em(metrics.y_size),
            horizontal_offset: to_em(metrics.x_offset),
            vertical_offset: -to_em(metrics.y_offset),
        });

        let superscript = ttf.superscript_metrics().map(|metrics| ScriptMetrics {
            width: to_em(metrics.x_size),
            height: to_em(metrics.y_size),
            horizontal_offset: to_em(metrics.x_offset),
            vertical_offset: to_em(metrics.y_offset),
        });

        Self {
            units_per_em,
            ascender,
            cap_height,
            x_height,
            descender,
            strikethrough,
            underline,
            overline,
            superscript,
            subscript,
            math: OnceLock::new(),
        }
    }

    fn init_math(font: &Font) -> Box<MathConstants> {
        let ttf = font.ttf();
        let metrics = font.metrics();

        let space_width = ttf
            .glyph_index(' ')
            .and_then(|id| ttf.glyph_hor_advance(id).map(|units| font.to_em(units)))
            .unwrap_or(typst_library::math::THICK);

        let is_cambria = || {
            font.find_name(name_id::POST_SCRIPT_NAME)
                .is_some_and(|name| name == "CambriaMath")
        };

        Box::new(
            ttf.tables()
                .math
                .and_then(|math| math.constants)
                .map(|constants| MathConstants {
                    space_width,
                    script_percent_scale_down: constants.script_percent_scale_down(),
                    script_script_percent_scale_down: constants
                        .script_script_percent_scale_down(),
                    display_operator_min_height: font.to_em(if is_cambria() {
                        constants.delimited_sub_formula_min_height()
                    } else {
                        constants.display_operator_min_height()
                    }),
                    axis_height: font.to_em(constants.axis_height().value),
                    accent_base_height: font.to_em(constants.accent_base_height().value),
                    flattened_accent_base_height: font
                        .to_em(constants.flattened_accent_base_height().value),
                    subscript_shift_down: font
                        .to_em(constants.subscript_shift_down().value),
                    subscript_top_max: font.to_em(constants.subscript_top_max().value),
                    subscript_baseline_drop_min: font
                        .to_em(constants.subscript_baseline_drop_min().value),
                    superscript_shift_up: font
                        .to_em(constants.superscript_shift_up().value),
                    superscript_shift_up_cramped: font
                        .to_em(constants.superscript_shift_up_cramped().value),
                    superscript_bottom_min: font
                        .to_em(constants.superscript_bottom_min().value),
                    superscript_baseline_drop_max: font
                        .to_em(constants.superscript_baseline_drop_max().value),
                    sub_superscript_gap_min: font
                        .to_em(constants.sub_superscript_gap_min().value),
                    superscript_bottom_max_with_subscript: font
                        .to_em(constants.superscript_bottom_max_with_subscript().value),
                    space_after_script: font.to_em(constants.space_after_script().value),
                    upper_limit_gap_min: font
                        .to_em(constants.upper_limit_gap_min().value),
                    upper_limit_baseline_rise_min: font
                        .to_em(constants.upper_limit_baseline_rise_min().value),
                    lower_limit_gap_min: font
                        .to_em(constants.lower_limit_gap_min().value),
                    lower_limit_baseline_drop_min: font
                        .to_em(constants.lower_limit_baseline_drop_min().value),
                    fraction_numerator_shift_up: font
                        .to_em(constants.fraction_numerator_shift_up().value),
                    fraction_numerator_display_style_shift_up: font.to_em(
                        constants.fraction_numerator_display_style_shift_up().value,
                    ),
                    fraction_denominator_shift_down: font
                        .to_em(constants.fraction_denominator_shift_down().value),
                    fraction_denominator_display_style_shift_down: font.to_em(
                        constants.fraction_denominator_display_style_shift_down().value,
                    ),
                    fraction_numerator_gap_min: font
                        .to_em(constants.fraction_numerator_gap_min().value),
                    fraction_num_display_style_gap_min: font
                        .to_em(constants.fraction_num_display_style_gap_min().value),
                    fraction_rule_thickness: font
                        .to_em(constants.fraction_rule_thickness().value),
                    fraction_denominator_gap_min: font
                        .to_em(constants.fraction_denominator_gap_min().value),
                    fraction_denom_display_style_gap_min: font
                        .to_em(constants.fraction_denom_display_style_gap_min().value),
                    skewed_fraction_vertical_gap: font
                        .to_em(constants.skewed_fraction_vertical_gap().value),
                    skewed_fraction_horizontal_gap: font
                        .to_em(constants.skewed_fraction_horizontal_gap().value),
                    overbar_vertical_gap: font
                        .to_em(constants.overbar_vertical_gap().value),
                    overbar_rule_thickness: font
                        .to_em(constants.overbar_rule_thickness().value),
                    overbar_extra_ascender: font
                        .to_em(constants.overbar_extra_ascender().value),
                    underbar_vertical_gap: font
                        .to_em(constants.underbar_vertical_gap().value),
                    underbar_rule_thickness: font
                        .to_em(constants.underbar_rule_thickness().value),
                    underbar_extra_descender: font
                        .to_em(constants.underbar_extra_descender().value),
                    radical_vertical_gap: font
                        .to_em(constants.radical_vertical_gap().value),
                    radical_display_style_vertical_gap: font
                        .to_em(constants.radical_display_style_vertical_gap().value),
                    radical_rule_thickness: font
                        .to_em(constants.radical_rule_thickness().value),
                    radical_extra_ascender: font
                        .to_em(constants.radical_extra_ascender().value),
                    radical_kern_before_degree: font
                        .to_em(constants.radical_kern_before_degree().value),
                    radical_kern_after_degree: font
                        .to_em(constants.radical_kern_after_degree().value),
                    radical_degree_bottom_raise_percent: constants
                        .radical_degree_bottom_raise_percent()
                        as f64
                        / 100.0,
                })
                // Most of these fallback constants are from the MathML Core
                // spec, with the exceptions of
                // - `flattened_accent_base_height` from Building Math Fonts
                // - `overbar_rule_thickness` and `underbar_rule_thickness`
                //   from our best guess
                // - `skewed_fraction_vertical_gap` and `skewed_fraction_horizontal_gap`
                //   from our best guess
                // - `script_percent_scale_down` and
                //   `script_script_percent_scale_down` from Building Math
                //   Fonts as the defaults given in MathML Core have more
                //   precision than i16.
                //
                // https://www.w3.org/TR/mathml-core/#layout-constants-mathconstants
                // https://github.com/notofonts/math/blob/main/documentation/building-math-fonts/index.md
                .unwrap_or(MathConstants {
                    space_width,
                    script_percent_scale_down: 70,
                    script_script_percent_scale_down: 50,
                    display_operator_min_height: Em::zero(),
                    axis_height: metrics.x_height / 2.0,
                    accent_base_height: metrics.x_height,
                    flattened_accent_base_height: metrics.cap_height,
                    subscript_shift_down: metrics
                        .subscript
                        .map(|metrics| metrics.vertical_offset)
                        .unwrap_or(DEFAULT_SUBSCRIPT_METRICS.vertical_offset),
                    subscript_top_max: 0.8 * metrics.x_height,
                    subscript_baseline_drop_min: Em::zero(),
                    superscript_shift_up: metrics
                        .superscript
                        .map(|metrics| metrics.vertical_offset)
                        .unwrap_or(DEFAULT_SUPERSCRIPT_METRICS.vertical_offset),
                    superscript_shift_up_cramped: Em::zero(),
                    superscript_bottom_min: 0.25 * metrics.x_height,
                    superscript_baseline_drop_max: Em::zero(),
                    sub_superscript_gap_min: 4.0 * metrics.underline.thickness,
                    superscript_bottom_max_with_subscript: 0.8 * metrics.x_height,
                    space_after_script: Em::new(1.0 / 24.0),
                    upper_limit_gap_min: Em::zero(),
                    upper_limit_baseline_rise_min: Em::zero(),
                    lower_limit_gap_min: Em::zero(),
                    lower_limit_baseline_drop_min: Em::zero(),
                    fraction_numerator_shift_up: Em::zero(),
                    fraction_numerator_display_style_shift_up: Em::zero(),
                    fraction_denominator_shift_down: Em::zero(),
                    fraction_denominator_display_style_shift_down: Em::zero(),
                    fraction_numerator_gap_min: metrics.underline.thickness,
                    fraction_num_display_style_gap_min: 3.0 * metrics.underline.thickness,
                    fraction_rule_thickness: metrics.underline.thickness,
                    fraction_denominator_gap_min: metrics.underline.thickness,
                    fraction_denom_display_style_gap_min: 3.0
                        * metrics.underline.thickness,
                    skewed_fraction_vertical_gap: Em::zero(),
                    skewed_fraction_horizontal_gap: Em::new(0.5),
                    overbar_vertical_gap: 3.0 * metrics.underline.thickness,
                    overbar_rule_thickness: metrics.underline.thickness,
                    overbar_extra_ascender: metrics.underline.thickness,
                    underbar_vertical_gap: 3.0 * metrics.underline.thickness,
                    underbar_rule_thickness: metrics.underline.thickness,
                    underbar_extra_descender: metrics.underline.thickness,
                    radical_vertical_gap: 1.25 * metrics.underline.thickness,
                    radical_display_style_vertical_gap: metrics.underline.thickness
                        + 0.25 * metrics.x_height,
                    radical_rule_thickness: metrics.underline.thickness,
                    radical_extra_ascender: metrics.underline.thickness,
                    radical_kern_before_degree: Em::new(5.0 / 18.0),
                    radical_kern_after_degree: Em::new(-10.0 / 18.0),
                    radical_degree_bottom_raise_percent: 0.6,
                }),
        )
    }

    /// Look up a vertical metric.
    pub fn vertical(&self, metric: VerticalFontMetric) -> Em {
        match metric {
            VerticalFontMetric::Ascender => self.ascender,
            VerticalFontMetric::CapHeight => self.cap_height,
            VerticalFontMetric::XHeight => self.x_height,
            VerticalFontMetric::Baseline => Em::zero(),
            VerticalFontMetric::Descender => self.descender,
        }
    }
}

/// Metrics for a decorative line.
#[derive(Debug, Copy, Clone)]
pub struct LineMetrics {
    /// The vertical offset of the line from the baseline. Positive goes
    /// upwards, negative downwards.
    pub position: Em,
    /// The thickness of the line.
    pub thickness: Em,
}

/// Metrics for subscripts or superscripts.
#[derive(Debug, Copy, Clone)]
pub struct ScriptMetrics {
    /// The width of those scripts, relative to the outer font size.
    pub width: Em,
    /// The height of those scripts, relative to the outer font size.
    pub height: Em,
    /// The horizontal (to the right) offset of those scripts, relative to the
    /// outer font size.
    ///
    /// This is used for italic correction.
    pub horizontal_offset: Em,
    /// The vertical (to the top) offset of those scripts, relative to the outer font size.
    ///
    /// For superscripts, this is positive. For subscripts, this is negative.
    pub vertical_offset: Em,
}

/// Constants from the OpenType MATH constants table used in Typst.
///
/// Ones not currently used are omitted.
#[derive(Debug, Copy, Clone)]
pub struct MathConstants {
    // This is not from the OpenType MATH spec.
    pub space_width: Em,
    // These are both i16 instead of f64 as they need to go on the StyleChain.
    pub script_percent_scale_down: i16,
    pub script_script_percent_scale_down: i16,
    pub display_operator_min_height: Em,
    pub axis_height: Em,
    pub accent_base_height: Em,
    pub flattened_accent_base_height: Em,
    pub subscript_shift_down: Em,
    pub subscript_top_max: Em,
    pub subscript_baseline_drop_min: Em,
    pub superscript_shift_up: Em,
    pub superscript_shift_up_cramped: Em,
    pub superscript_bottom_min: Em,
    pub superscript_baseline_drop_max: Em,
    pub sub_superscript_gap_min: Em,
    pub superscript_bottom_max_with_subscript: Em,
    pub space_after_script: Em,
    pub upper_limit_gap_min: Em,
    pub upper_limit_baseline_rise_min: Em,
    pub lower_limit_gap_min: Em,
    pub lower_limit_baseline_drop_min: Em,
    pub fraction_numerator_shift_up: Em,
    pub fraction_numerator_display_style_shift_up: Em,
    pub fraction_denominator_shift_down: Em,
    pub fraction_denominator_display_style_shift_down: Em,
    pub fraction_numerator_gap_min: Em,
    pub fraction_num_display_style_gap_min: Em,
    pub fraction_rule_thickness: Em,
    pub fraction_denominator_gap_min: Em,
    pub fraction_denom_display_style_gap_min: Em,
    pub skewed_fraction_vertical_gap: Em,
    pub skewed_fraction_horizontal_gap: Em,
    pub overbar_vertical_gap: Em,
    pub overbar_rule_thickness: Em,
    pub overbar_extra_ascender: Em,
    pub underbar_vertical_gap: Em,
    pub underbar_rule_thickness: Em,
    pub underbar_extra_descender: Em,
    pub radical_vertical_gap: Em,
    pub radical_display_style_vertical_gap: Em,
    pub radical_rule_thickness: Em,
    pub radical_extra_ascender: Em,
    pub radical_kern_before_degree: Em,
    pub radical_kern_after_degree: Em,
    pub radical_degree_bottom_raise_percent: f64,
}

/// Identifies a vertical metric of a font.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum VerticalFontMetric {
    /// The font's ascender, which typically exceeds the height of all glyphs.
    Ascender,
    /// The approximate height of uppercase letters.
    CapHeight,
    /// The approximate height of non-ascending lowercase letters.
    XHeight,
    /// The baseline on which the letters rest.
    Baseline,
    /// The font's ascender, which typically exceeds the depth of all glyphs.
    Descender,
}

/// Defines how to resolve a `Bounds` text edge.
#[derive(Debug, Copy, Clone)]
pub enum TextEdgeBounds<'a> {
    /// Set the bounds to zero.
    Zero,
    /// Use the bounding box of the given glyph for the bounds.
    Glyph(u16),
    /// Use the dimension of the given frame for the bounds.
    Frame(&'a Frame),
}

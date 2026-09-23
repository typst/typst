use std::sync::OnceLock;

use skrifa::MetadataProvider;
use skrifa::raw::TableProvider;

use crate::foundations::Cast;
use crate::layout::{Em, Frame};
use crate::text::{DEFAULT_SUBSCRIPT_METRICS, DEFAULT_SUPERSCRIPT_METRICS, FontInstance};

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
    pub fn from_skrifa(
        skrifa: &skrifa::FontRef,
        location: skrifa::instance::LocationRef,
    ) -> Self {
        let metrics = skrifa.metrics(skrifa::instance::Size::unscaled(), location);

        let units_per_em = f64::from(metrics.units_per_em);
        let to_em = |units| Em::from_units(units, units_per_em);
        let coords = location.coords();

        let ascender = to_em(
            skrifa
                .os2()
                .ok()
                .map(|os2| {
                    let mut ascent = os2.s_typo_ascender() as f32;
                    if let (Ok(mvar), true) = (skrifa.mvar(), !coords.is_empty()) {
                        use skrifa::raw::tables::mvar::tags::*;
                        let metric_delta = |tag| {
                            mvar.metric_delta(tag, coords).unwrap_or_default().to_f64()
                                as f32
                        };
                        ascent += metric_delta(HASC);
                    }

                    ascent
                })
                .unwrap_or(metrics.ascent),
        );
        let cap_height = metrics.cap_height.filter(|&h| h > 0.0).map_or(ascender, to_em);
        let x_height = metrics.x_height.filter(|&h| h > 0.0).map_or(ascender, to_em);
        let descender = to_em(
            skrifa
                .os2()
                .ok()
                .map(|os2| {
                    let mut descent = os2.s_typo_descender() as f32;
                    if let (Ok(mvar), true) = (skrifa.mvar(), !coords.is_empty()) {
                        use skrifa::raw::tables::mvar::tags::*;
                        let metric_delta = |tag| {
                            mvar.metric_delta(tag, coords).unwrap_or_default().to_f64()
                                as f32
                        };
                        descent += metric_delta(HDSC);
                    }

                    descent
                })
                .unwrap_or(metrics.descent),
        );

        let strikeout = metrics.strikeout;
        let underline = metrics.underline;

        let strikethrough = LineMetrics {
            position: strikeout.map_or(Em::new(0.25), |s| to_em(s.offset)),
            thickness: strikeout
                .or(underline)
                .map_or(Em::new(0.06), |s| to_em(s.thickness)),
        };

        let underline = LineMetrics {
            position: underline.map_or(Em::new(-0.2), |s| to_em(s.offset)),
            thickness: underline
                .or(strikeout)
                .map_or(Em::new(0.06), |s| to_em(s.thickness)),
        };

        let overline = LineMetrics {
            position: cap_height + Em::new(0.1),
            thickness: underline.thickness,
        };

        let subscript = skrifa.os2().ok().map(|os2| {
            let mut x_size = os2.y_subscript_x_size() as f32;
            let mut y_size = os2.y_subscript_y_size() as f32;
            let mut x_offset = os2.y_subscript_x_offset() as f32;
            let mut y_offset = os2.y_subscript_y_offset() as f32;

            if let (Ok(mvar), true) = (skrifa.mvar(), !coords.is_empty()) {
                use skrifa::raw::tables::mvar::tags::*;
                let metric_delta = |tag| {
                    mvar.metric_delta(tag, coords).unwrap_or_default().to_f64() as f32
                };
                x_size += metric_delta(SBXS);
                y_size += metric_delta(SBYS);
                x_offset += metric_delta(SBXO);
                y_offset += metric_delta(SBYO);
            }

            ScriptMetrics {
                width: to_em(x_size),
                height: to_em(y_size),
                horizontal_offset: to_em(x_offset),
                vertical_offset: -to_em(y_offset),
            }
        });

        let superscript = skrifa.os2().ok().map(|os2| {
            let mut x_size = os2.y_superscript_x_size() as f32;
            let mut y_size = os2.y_superscript_y_size() as f32;
            let mut x_offset = os2.y_superscript_x_offset() as f32;
            let mut y_offset = os2.y_superscript_y_offset() as f32;

            if let (Ok(mvar), true) = (skrifa.mvar(), !coords.is_empty()) {
                use skrifa::raw::tables::mvar::tags::*;
                let metric_delta = |tag| {
                    mvar.metric_delta(tag, coords).unwrap_or_default().to_f64() as f32
                };
                x_size += metric_delta(SPXS);
                y_size += metric_delta(SPYS);
                x_offset += metric_delta(SPXO);
                y_offset += metric_delta(SPYO);
            }

            ScriptMetrics {
                width: to_em(x_size),
                height: to_em(y_size),
                horizontal_offset: to_em(x_offset),
                vertical_offset: to_em(y_offset),
            }
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
    pub stack_top_shift_up: Em,
    pub stack_top_display_style_shift_up: Em,
    pub stack_bottom_shift_down: Em,
    pub stack_bottom_display_style_shift_down: Em,
    pub stack_gap_min: Em,
    pub stack_display_style_gap_min: Em,
    pub stretch_stack_top_shift_up: Em,
    pub stretch_stack_bottom_shift_down: Em,
    pub stretch_stack_gap_above_min: Em,
    pub stretch_stack_gap_below_min: Em,
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

impl MathConstants {
    pub(super) fn new(font: &FontInstance) -> Box<Self> {
        let space_width = font
            .glyph_index(' ')
            .and_then(|id| font.x_advance(id))
            .unwrap_or(typst_library::math::THICK);

        font.skrifa()
            .math()
            .ok()
            .and_then(|math| {
                Some((math.math_constants().ok()?, math.has_swapped_min_heights()))
            })
            .map(|(constants, is_cambria)| {
                Self::from_constants(font, &constants, space_width, is_cambria)
            })
            .unwrap_or_else(|| Self::fallback(font, space_width))
    }

    fn from_constants(
        font: &FontInstance,
        constants: &skrifa::raw::tables::math::MathConstants,
        space_width: Em,
        is_cambria: bool,
    ) -> Box<Self> {
        Box::new(Self {
            space_width,
            script_percent_scale_down: constants.script_percent_scale_down(),
            script_script_percent_scale_down: constants
                .script_script_percent_scale_down(),
            display_operator_min_height: font.to_em(if is_cambria {
                constants.delimited_sub_formula_min_height().to_u16()
            } else {
                constants.display_operator_min_height().to_u16()
            }),
            axis_height: font.to_em(constants.axis_height().value().to_i16()),
            accent_base_height: font
                .to_em(constants.accent_base_height().value().to_i16()),
            flattened_accent_base_height: font
                .to_em(constants.flattened_accent_base_height().value().to_i16()),
            subscript_shift_down: font
                .to_em(constants.subscript_shift_down().value().to_i16()),
            subscript_top_max: font.to_em(constants.subscript_top_max().value().to_i16()),
            subscript_baseline_drop_min: font
                .to_em(constants.subscript_baseline_drop_min().value().to_i16()),
            superscript_shift_up: font
                .to_em(constants.superscript_shift_up().value().to_i16()),
            superscript_shift_up_cramped: font
                .to_em(constants.superscript_shift_up_cramped().value().to_i16()),
            superscript_bottom_min: font
                .to_em(constants.superscript_bottom_min().value().to_i16()),
            superscript_baseline_drop_max: font
                .to_em(constants.superscript_baseline_drop_max().value().to_i16()),
            sub_superscript_gap_min: font
                .to_em(constants.sub_superscript_gap_min().value().to_i16()),
            superscript_bottom_max_with_subscript: font.to_em(
                constants.superscript_bottom_max_with_subscript().value().to_i16(),
            ),
            space_after_script: font
                .to_em(constants.space_after_script().value().to_i16()),
            upper_limit_gap_min: font
                .to_em(constants.upper_limit_gap_min().value().to_i16()),
            upper_limit_baseline_rise_min: font
                .to_em(constants.upper_limit_baseline_rise_min().value().to_i16()),
            lower_limit_gap_min: font
                .to_em(constants.lower_limit_gap_min().value().to_i16()),
            lower_limit_baseline_drop_min: font
                .to_em(constants.lower_limit_baseline_drop_min().value().to_i16()),
            stack_top_shift_up: font
                .to_em(constants.stack_top_shift_up().value().to_i16()),
            stack_top_display_style_shift_up: font
                .to_em(constants.stack_top_display_style_shift_up().value().to_i16()),
            stack_bottom_shift_down: font
                .to_em(constants.stack_bottom_shift_down().value().to_i16()),
            stack_bottom_display_style_shift_down: font.to_em(
                constants.stack_bottom_display_style_shift_down().value().to_i16(),
            ),
            stack_gap_min: font.to_em(constants.stack_gap_min().value().to_i16()),
            stack_display_style_gap_min: font
                .to_em(constants.stack_display_style_gap_min().value().to_i16()),
            stretch_stack_top_shift_up: font
                .to_em(constants.stretch_stack_top_shift_up().value().to_i16()),
            stretch_stack_bottom_shift_down: font
                .to_em(constants.stretch_stack_bottom_shift_down().value().to_i16()),
            stretch_stack_gap_above_min: font
                .to_em(constants.stretch_stack_gap_above_min().value().to_i16()),
            stretch_stack_gap_below_min: font
                .to_em(constants.stretch_stack_gap_below_min().value().to_i16()),
            fraction_numerator_shift_up: font
                .to_em(constants.fraction_numerator_shift_up().value().to_i16()),
            fraction_numerator_display_style_shift_up: font.to_em(
                constants.fraction_numerator_display_style_shift_up().value().to_i16(),
            ),
            fraction_denominator_shift_down: font
                .to_em(constants.fraction_denominator_shift_down().value().to_i16()),
            fraction_denominator_display_style_shift_down: font.to_em(
                constants
                    .fraction_denominator_display_style_shift_down()
                    .value()
                    .to_i16(),
            ),
            fraction_numerator_gap_min: font
                .to_em(constants.fraction_numerator_gap_min().value().to_i16()),
            fraction_num_display_style_gap_min: font
                .to_em(constants.fraction_num_display_style_gap_min().value().to_i16()),
            fraction_rule_thickness: font
                .to_em(constants.fraction_rule_thickness().value().to_i16()),
            fraction_denominator_gap_min: font
                .to_em(constants.fraction_denominator_gap_min().value().to_i16()),
            fraction_denom_display_style_gap_min: font
                .to_em(constants.fraction_denom_display_style_gap_min().value().to_i16()),
            skewed_fraction_vertical_gap: font
                .to_em(constants.skewed_fraction_vertical_gap().value().to_i16()),
            skewed_fraction_horizontal_gap: font
                .to_em(constants.skewed_fraction_horizontal_gap().value().to_i16()),
            overbar_vertical_gap: font
                .to_em(constants.overbar_vertical_gap().value().to_i16()),
            overbar_rule_thickness: font
                .to_em(constants.overbar_rule_thickness().value().to_i16()),
            overbar_extra_ascender: font
                .to_em(constants.overbar_extra_ascender().value().to_i16()),
            underbar_vertical_gap: font
                .to_em(constants.underbar_vertical_gap().value().to_i16()),
            underbar_rule_thickness: font
                .to_em(constants.underbar_rule_thickness().value().to_i16()),
            underbar_extra_descender: font
                .to_em(constants.underbar_extra_descender().value().to_i16()),
            radical_vertical_gap: font
                .to_em(constants.radical_vertical_gap().value().to_i16()),
            radical_display_style_vertical_gap: font
                .to_em(constants.radical_display_style_vertical_gap().value().to_i16()),
            radical_rule_thickness: font
                .to_em(constants.radical_rule_thickness().value().to_i16()),
            radical_extra_ascender: font
                .to_em(constants.radical_extra_ascender().value().to_i16()),
            radical_kern_before_degree: font
                .to_em(constants.radical_kern_before_degree().value().to_i16()),
            radical_kern_after_degree: font
                .to_em(constants.radical_kern_after_degree().value().to_i16()),
            radical_degree_bottom_raise_percent: constants
                .radical_degree_bottom_raise_percent()
                as f64
                / 100.0,
        })
    }

    /// Most of these fallback constants are from the MathML Core
    /// spec, with the exceptions of
    /// - `flattened_accent_base_height` from Building Math Fonts
    /// - `overbar_rule_thickness` and `underbar_rule_thickness`
    ///   from our best guess
    /// - `skewed_fraction_vertical_gap` and `skewed_fraction_horizontal_gap`
    ///   from our best guess
    /// - `script_percent_scale_down` and
    ///   `script_script_percent_scale_down` from Building Math
    ///   Fonts as the defaults given in MathML Core have more
    ///   precision than i16.
    ///
    /// <https://www.w3.org/TR/mathml-core/#layout-constants-mathconstants>
    /// <https://github.com/notofonts/math/blob/main/documentation/building-math-fonts/index.md>
    fn fallback(font: &FontInstance, space_width: Em) -> Box<Self> {
        let metrics = font.metrics();
        Box::new(MathConstants {
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
            stack_top_shift_up: Em::zero(),
            stack_top_display_style_shift_up: Em::zero(),
            stack_bottom_shift_down: Em::zero(),
            stack_bottom_display_style_shift_down: Em::zero(),
            stack_gap_min: 3.0 * metrics.underline.thickness,
            stack_display_style_gap_min: 7.0 * metrics.underline.thickness,
            stretch_stack_top_shift_up: Em::zero(),
            stretch_stack_bottom_shift_down: Em::zero(),
            stretch_stack_gap_above_min: Em::zero(),
            stretch_stack_gap_below_min: Em::zero(),
            fraction_numerator_shift_up: Em::zero(),
            fraction_numerator_display_style_shift_up: Em::zero(),
            fraction_denominator_shift_down: Em::zero(),
            fraction_denominator_display_style_shift_down: Em::zero(),
            fraction_numerator_gap_min: metrics.underline.thickness,
            fraction_num_display_style_gap_min: 3.0 * metrics.underline.thickness,
            fraction_rule_thickness: metrics.underline.thickness,
            fraction_denominator_gap_min: metrics.underline.thickness,
            fraction_denom_display_style_gap_min: 3.0 * metrics.underline.thickness,
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
        })
    }
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
    Glyph(u32),
    /// Use the dimension of the given frame for the bounds.
    Frame(&'a Frame),
}

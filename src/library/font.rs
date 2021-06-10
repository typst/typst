use crate::font::{FontStretch, FontStyle, FontWeight};
use crate::layout::Fill;

use super::*;

/// `font`: Configure the font.
///
/// # Positional parameters
/// - Font size: optional, of type `linear` relative to current font size.
/// - Font families: variadic, of type `font-family`.
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Font Style: `style`, of type `font-style`.
/// - Font Weight: `weight`, of type `font-weight`.
/// - Font Stretch: `stretch`, of type `relative`, between 0.5 and 2.0.
/// - Top edge of the font: `top-edge`, of type `vertical-font-metric`.
/// - Bottom edge of the font: `bottom-edge`, of type `vertical-font-metric`.
/// - Color the glyphs: `color`, of type `color`.
/// - Serif family definition: `serif`, of type `font-family-definition`.
/// - Sans-serif family definition: `sans-serif`, of type `font-family-definition`.
/// - Monospace family definition: `monospace`, of type `font-family-definition`.
///
/// # Return value
/// A template that configures font properties. The effect is scoped to the body
/// if present.
///
/// # Relevant types and constants
/// - Type `font-family`
///   - `serif`
///   - `sans-serif`
///   - `monospace`
///   - coerces from `string`
/// - Type `font-family-definition`
///   - coerces from `string`
///   - coerces from `array`
/// - Type `font-style`
///   - `normal`
///   - `italic`
///   - `oblique`
/// - Type `font-weight`
///   - `regular` (400)
///   - `bold` (700)
///   - coerces from `integer`, between 100 and 900
/// - Type `vertical-font-metric`
///   - `ascender`
///   - `cap-height`
///   - `x-height`
///   - `baseline`
///   - `descender`
pub fn font(ctx: &mut EvalContext, args: &mut FuncArgs) -> Value {
    let size = args.eat::<Linear>(ctx);
    let list = args.eat_all::<FontFamily>(ctx);
    let style = args.eat_named(ctx, "style");
    let weight = args.eat_named(ctx, "weight");
    let stretch = args.eat_named(ctx, "stretch");
    let top_edge = args.eat_named(ctx, "top-edge");
    let bottom_edge = args.eat_named(ctx, "bottom-edge");
    let color = args.eat_named(ctx, "color");
    let serif = args.eat_named(ctx, "serif");
    let sans_serif = args.eat_named(ctx, "sans-serif");
    let monospace = args.eat_named(ctx, "monospace");
    let body = args.eat::<TemplateValue>(ctx);

    Value::template("font", move |ctx| {
        let snapshot = ctx.state.clone();

        if let Some(linear) = size {
            if linear.rel.is_zero() {
                ctx.state.font.size = linear.abs;
                ctx.state.font.scale = Linear::one();
            } else {
                ctx.state.font.scale = linear;
            }
        }

        if !list.is_empty() {
            ctx.state.font.families_mut().list = list.clone();
        }

        if let Some(style) = style {
            ctx.state.font.variant.style = style;
        }

        if let Some(weight) = weight {
            ctx.state.font.variant.weight = weight;
        }

        if let Some(stretch) = stretch {
            ctx.state.font.variant.stretch = stretch;
        }

        if let Some(top_edge) = top_edge {
            ctx.state.font.top_edge = top_edge;
        }

        if let Some(bottom_edge) = bottom_edge {
            ctx.state.font.bottom_edge = bottom_edge;
        }

        if let Some(color) = color {
            ctx.state.font.fill = Fill::Color(color);
        }

        if let Some(FontFamilies(serif)) = &serif {
            ctx.state.font.families_mut().serif = serif.clone();
        }

        if let Some(FontFamilies(sans_serif)) = &sans_serif {
            ctx.state.font.families_mut().sans_serif = sans_serif.clone();
        }

        if let Some(FontFamilies(monospace)) = &monospace {
            ctx.state.font.families_mut().monospace = monospace.clone();
        }

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}

/// A list of font family names.
#[derive(Debug, Clone, PartialEq)]
struct FontFamilies(Vec<String>);

value! {
    FontFamilies: "string or array of strings",
    Value::Str(string) => Self(vec![string.to_lowercase()]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: String| string.to_lowercase())
        .collect()
    ),
}

value! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(string.to_lowercase())
}

value! {
    FontStyle: "font style",
}

value! {
    FontWeight: "font weight",
    Value::Int(number) => {
        let [min, max] = [Self::THIN, Self::BLACK];
        let message = || format!(
            "should be between {} and {}",
            min.to_number(),
            max.to_number(),
        );

        return if number < i64::from(min.to_number()) {
            CastResult::Warn(min, message())
        } else if number > i64::from(max.to_number()) {
            CastResult::Warn(max, message())
        } else {
            CastResult::Ok(Self::from_number(number as u16))
        };
    },
}

value! {
    FontStretch: "font stretch",
    Value::Relative(relative) => {
        let [min, max] = [Self::ULTRA_CONDENSED, Self::ULTRA_EXPANDED];
        let message = || format!(
            "should be between {} and {}",
            Relative::new(min.to_ratio() as f64),
            Relative::new(max.to_ratio() as f64),
        );

        let ratio = relative.get() as f32;
        let value = Self::from_ratio(ratio);

        return if ratio < min.to_ratio() || ratio > max.to_ratio() {
            CastResult::Warn(value, message())
        } else {
            CastResult::Ok(value)
        };
    },
}

value! {
    VerticalFontMetric: "vertical font metric",
}

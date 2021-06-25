use crate::font::{FontStretch, FontStyle, FontWeight};
use crate::layout::Fill;

use super::*;

/// `font`: Configure the font.
///
/// # Positional parameters
/// - Body: optional, of type `template`.
///
/// # Named parameters
/// - Font size: `size`, of type `linear` relative to current font size.
/// - Font families: `family`, `font-family`, `string` or `array`.
/// - Font Style: `style`, of type `font-style`.
/// - Font Weight: `weight`, of type `font-weight`.
/// - Font Stretch: `stretch`, of type `relative`, between 0.5 and 2.0.
/// - Top edge of the font: `top-edge`, of type `vertical-font-metric`.
/// - Bottom edge of the font: `bottom-edge`, of type `vertical-font-metric`.
/// - Color the glyphs: `color`, of type `color`.
/// - Serif family definition: `serif`, of type `family-def`.
/// - Sans-serif family definition: `sans-serif`, of type `family-def`.
/// - Monospace family definition: `monospace`, of type `family-def`.
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
/// - Type `family-def`
///   - coerces from `string`
///   - coerces from `array` of `string`
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
    let list = args.named(ctx, "family");
    let size = args.named::<Linear>(ctx, "size");
    let style = args.named(ctx, "style");
    let weight = args.named(ctx, "weight");
    let stretch = args.named(ctx, "stretch");
    let top_edge = args.named(ctx, "top-edge");
    let bottom_edge = args.named(ctx, "bottom-edge");
    let color = args.named(ctx, "color");
    let serif = args.named(ctx, "serif");
    let sans_serif = args.named(ctx, "sans-serif");
    let monospace = args.named(ctx, "monospace");
    let body = args.eat::<TemplateValue>(ctx);

    Value::template("font", move |ctx| {
        let snapshot = ctx.state.clone();
        let font = ctx.state.font_mut();

        if let Some(linear) = size {
            font.size = linear.resolve(font.size);
        }

        if let Some(FontDef(list)) = &list {
            font.families_mut().list = list.clone();
        }

        if let Some(style) = style {
            font.variant.style = style;
        }

        if let Some(weight) = weight {
            font.variant.weight = weight;
        }

        if let Some(stretch) = stretch {
            font.variant.stretch = stretch;
        }

        if let Some(top_edge) = top_edge {
            font.top_edge = top_edge;
        }

        if let Some(bottom_edge) = bottom_edge {
            font.bottom_edge = bottom_edge;
        }

        if let Some(color) = color {
            font.fill = Fill::Color(color);
        }

        if let Some(FamilyDef(serif)) = &serif {
            font.families_mut().serif = serif.clone();
        }

        if let Some(FamilyDef(sans_serif)) = &sans_serif {
            font.families_mut().sans_serif = sans_serif.clone();
        }

        if let Some(FamilyDef(monospace)) = &monospace {
            font.families_mut().monospace = monospace.clone();
        }

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}

#[derive(Debug)]
struct FontDef(Vec<FontFamily>);

castable! {
    FontDef: "font family or array of font families",
    Value::Str(string) => Self(vec![FontFamily::Named(string.to_lowercase())]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
    #(family: FontFamily) => Self(vec![family]),
}

#[derive(Debug)]
struct FamilyDef(Vec<String>);

castable! {
    FamilyDef: "string or array of strings",
    Value::Str(string) => Self(vec![string.to_lowercase()]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .map(|string: String| string.to_lowercase())
        .collect()
    ),
}

castable! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(string.to_lowercase())
}

castable! {
    FontStyle: "font style",
}

castable! {
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

castable! {
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

castable! {
    VerticalFontMetric: "vertical font metric",
}

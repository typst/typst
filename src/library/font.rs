use fontdock::{FontStretch, FontStyle, FontWeight};

use super::*;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - Font size:     optional, of type `linear` relative to current font size.
/// - Font families: variadic, of type `font-family`.
/// - Body:          optional, of type `template`.
///
/// # Named arguments
/// - Font Style:                   `style`, of type `font-style`.
/// - Font Weight:                  `weight`, of type `font-weight`.
/// - Font Stretch:                 `stretch`, of type `font-stretch`.
/// - Serif family definition:      `serif`, of type `font-families`.
/// - Sans-serif family definition: `sans-serif`, of type `font-families`.
/// - Monospace family definition:  `monospace`, of type `font-families`.
///
/// # Relevant types and constants
/// - Type `font-families`
///     - coerces from `string`
///     - coerces from `array`
///     - coerces from `font-family`
/// - Type `font-family`
///     - `serif`
///     - `sans-serif`
///     - `monospace`
///     - coerces from `string`
/// - Type `font-style`
///     - `normal`
///     - `italic`
///     - `oblique`
/// - Type `font-weight`
///     - `thin` (100)
///     - `extralight` (200)
///     - `light` (300)
///     - `regular` (400)
///     - `medium` (500)
///     - `semibold` (600)
///     - `bold` (700)
///     - `extrabold` (800)
///     - `black` (900)
///     - coerces from `integer`
/// - Type `font-stretch`
///     - `ultra-condensed`
///     - `extra-condensed`
///     - `condensed`
///     - `semi-condensed`
///     - `normal`
///     - `semi-expanded`
///     - `expanded`
///     - `extra-expanded`
///     - `ultra-expanded`
pub fn font(ctx: &mut EvalContext, args: &mut ValueArgs) -> Value {
    let size = args.find::<Linear>(ctx);
    let list: Vec<_> = args.filter::<FontFamily>(ctx).map(|f| f.to_string()).collect();
    let style = args.get(ctx, "style");
    let weight = args.get(ctx, "weight");
    let stretch = args.get(ctx, "stretch");
    let serif = args.get(ctx, "serif");
    let sans_serif = args.get(ctx, "sans-serif");
    let monospace = args.get(ctx, "monospace");
    let body = args.find::<ValueTemplate>(ctx);

    Value::template("font", move |ctx| {
        let snapshot = ctx.state.clone();

        if let Some(linear) = size {
            if linear.rel.is_zero() {
                ctx.state.font.size = linear.abs;
                ctx.state.font.scale = Relative::ONE.into();
            } else {
                ctx.state.font.scale = linear;
            }
        }

        if !list.is_empty() {
            let families = ctx.state.font.families_mut();
            families.list = list.clone();
            families.flatten();
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

        for (variant, arg) in &[
            (FontFamily::Serif, &serif),
            (FontFamily::SansSerif, &sans_serif),
            (FontFamily::Monospace, &monospace),
        ] {
            if let Some(FontFamilies(list)) = arg {
                let strings = list.into_iter().map(|f| f.to_string()).collect();
                let families = ctx.state.font.families_mut();
                families.update_class_list(variant.to_string(), strings);
                families.flatten();
            }
        }

        if let Some(body) = &body {
            body.exec(ctx);
            ctx.state = snapshot;
        }
    })
}

/// A list of font families.
#[derive(Debug, Clone, PartialEq)]
struct FontFamilies(Vec<FontFamily>);

/// A single font family.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum FontFamily {
    Serif,
    SansSerif,
    Monospace,
    Named(String),
}

impl FontFamily {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Serif => "serif",
            Self::SansSerif => "sans-serif",
            Self::Monospace => "monospace",
            Self::Named(s) => s,
        }
    }
}

impl Display for FontFamily {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.as_str())
    }
}

typify! {
    FontFamilies: "font family or array of font families",
    Value::Str(string) => Self(vec![FontFamily::Named(string.to_lowercase())]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
    #(family: FontFamily) => Self(vec![family]),
}

typify! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(string.to_lowercase())
}

typify! {
    FontStyle: "font style"
}

typify! {
    FontWeight: "font weight",
    Value::Int(number) => {
        let [min, max] = [Self::THIN, Self::BLACK];
        let message = || format!("should be between {:#?} and {:#?}", min, max);
        return if number < i64::from(min.to_number()) {
            CastResult::Warn(min, message())
        } else if number > i64::from(max.to_number()) {
            CastResult::Warn(max, message())
        } else {
            CastResult::Ok(Self::from_number(number as u16))
        };
    },
}

typify! {
    FontStretch: "font stretch"
}

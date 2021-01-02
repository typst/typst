use std::fmt::{self, Display, Formatter};
use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontWeight};

use crate::color::{Color, RgbaColor};
use crate::prelude::*;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - Font size:     optional, of type `linear` relative to current font size.
/// - Font families: variadic, of type `font-family`.
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
pub fn font(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let snapshot = ctx.state.clone();

    if let Some(linear) = args.find::<Linear>(ctx) {
        if linear.is_absolute() {
            ctx.state.font.size = linear.abs;
            ctx.state.font.scale = Relative::ONE.into();
        } else {
            ctx.state.font.scale = linear;
        }
    }

    let list: Vec<_> = args.filter::<FontFamily>(ctx).map(|f| f.to_string()).collect();
    if !list.is_empty() {
        let families = Rc::make_mut(&mut ctx.state.font.families);
        families.list = list;
        families.flatten();
    }

    if let Some(style) = args.get(ctx, "style") {
        ctx.state.font.variant.style = style;
    }

    if let Some(weight) = args.get(ctx, "weight") {
        ctx.state.font.variant.weight = weight;
    }

    if let Some(stretch) = args.get(ctx, "stretch") {
        ctx.state.font.variant.stretch = stretch;
    }

    for variant in FontFamily::VARIANTS {
        if let Some(FontFamilies(list)) = args.get(ctx, variant.as_str()) {
            let strings = list.into_iter().map(|f| f.to_string()).collect();
            let families = Rc::make_mut(&mut ctx.state.font.families);
            families.update_class_list(variant.to_string(), strings);
            families.flatten();
        }
    }

    if let Some(body) = args.find::<ValueContent>(ctx) {
        body.eval(ctx);
        ctx.state = snapshot;
    }

    Value::None
}

/// A list of font families.
#[derive(Debug, Clone, PartialEq)]
struct FontFamilies(Vec<FontFamily>);

impl_type! {
    FontFamilies: "font family or array of font families",
    Value::Str(string) => Self(vec![FontFamily::Named(string.to_lowercase())]),
    Value::Array(values) => Self(values
        .into_iter()
        .filter_map(|v| v.cast().ok())
        .collect()
    ),
    #(family: FontFamily) => Self(vec![family]),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum FontFamily {
    Serif,
    SansSerif,
    Monospace,
    Named(String),
}

impl FontFamily {
    pub const VARIANTS: &'static [Self] =
        &[Self::Serif, Self::SansSerif, Self::Monospace];

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

impl_type! {
    FontFamily: "font family",
    Value::Str(string) => Self::Named(string.to_lowercase())
}

impl_type! {
    FontStyle: "font style"
}

impl_type! {
    FontWeight: "font weight",
    Value::Int(number) => {
        let [min, max] = [Self::THIN, Self::BLACK];
        let message = || format!("must be between {:#?} and {:#?}", min, max);
        return if number < i64::from(min.to_number()) {
            CastResult::Warn(min, message())
        } else if number > i64::from(max.to_number()) {
            CastResult::Warn(max, message())
        } else {
            CastResult::Ok(Self::from_number(number as u16))
        };
    },
}

impl_type! {
    FontStretch: "font stretch"
}

/// `rgb`: Create an RGB(A) color.
///
/// # Positional arguments
/// - Red component:   of type `float`, between 0.0 and 1.0.
/// - Green component: of type `float`, between 0.0 and 1.0.
/// - Blue component:  of type `float`, between 0.0 and 1.0.
/// - Alpha component: optional, of type `float`, between 0.0 and 1.0.
pub fn rgb(ctx: &mut EvalContext, args: &mut Args) -> Value {
    let r = args.require(ctx, "red component");
    let g = args.require(ctx, "green component");
    let b = args.require(ctx, "blue component");
    let a = args.find(ctx);

    let mut clamp = |component: Option<Spanned<f64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0.0 || c.v > 1.0 {
                ctx.diag(warning!(c.span, "must be between 0.0 and 1.0"));
            }
            (c.v.max(0.0).min(1.0) * 255.0).round() as u8
        })
    };

    Value::Color(Color::Rgba(RgbaColor::new(
        clamp(r, 0),
        clamp(g, 0),
        clamp(b, 0),
        clamp(a, 255),
    )))
}

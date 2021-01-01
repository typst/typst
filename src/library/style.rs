use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontWeight};

use crate::color::{Color, RgbaColor};
use crate::eval::StringLike;
use crate::geom::Linear;
use crate::prelude::*;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - Font size (optional, `linear` relative to current font size).
/// - Font families ... (optional, variadic, `Family`)
///
/// # Named arguments
/// - `style` (`Style`): The font style.
/// - `weight` (`Weight`): The font weight.
/// - `stretch` (`Stretch`): The font stretch.
/// - `serif` (`Family` or `dict` of type `Family`): The serif family.
/// - `sans-serif` (`Family` or `dict` of type `Family`): The new sansserif family.
/// - `monospace` (`Family` or `dict` of type `Family`): The monospace family.
/// - `emoji` (`Family` or `dict` of type `Family`): The emoji family.
/// - `math` (`Family` or `dict` of type `Family`): The math family.
///
/// # Examples
/// Set font size and font families.
/// ```typst
/// [font 12pt, "Arial", "Noto Sans", sans-serif]
/// ```
///
/// Redefine the default sans-serif family to a single font family.
/// ```typst
/// [font sans-serif: "Source Sans Pro"]
/// ```
///
/// Redefine the default emoji family with a fallback.
/// ```typst
/// [font emoji: ("Segoe UI Emoji", "Noto Emoji")]
/// ```
///
/// # Enumerations
/// - `Family`
///     - `serif`
///     - `sans-serif`
///     - `monospace`
///     - `emoji`
///     - `math`
///     - any string
/// - `Style`
///     - `normal`
///     - `italic`
///     - `oblique`
/// - `Weight`
///     - `thin` or `hairline` (100)
///     - `extralight` (200)
///     - `light` (300)
///     - `regular` (400)
///     - `medium` (500)
///     - `semibold` (600)
///     - `bold` (700)
///     - `extrabold` (800)
///     - `black` (900)
///     - any integer between 100 and 900
/// - `Stretch`
///     - `ultra-condensed`
///     - `extra-condensed`
///     - `condensed`
///     - `semi-condensed`
///     - `normal`
///     - `semi-expanded`
///     - `expanded`
///     - `extra-expanded`
///     - `ultra-expanded`
pub fn font(mut args: Args, ctx: &mut EvalContext) -> Value {
    let snapshot = ctx.state.clone();
    let body = args.find::<SynTree>();

    if let Some(linear) = args.find::<Linear>() {
        if linear.is_absolute() {
            ctx.state.font.size = linear.abs;
            ctx.state.font.scale = Relative::ONE.into();
        } else {
            ctx.state.font.scale = linear;
        }
    }

    let mut needs_flattening = false;
    let list: Vec<_> = args.find_all::<StringLike>().map(|s| s.to_lowercase()).collect();

    if !list.is_empty() {
        Rc::make_mut(&mut ctx.state.font.families).list = list;
        needs_flattening = true;
    }

    if let Some(style) = args.get::<_, FontStyle>(ctx, "style") {
        ctx.state.font.variant.style = style;
    }

    if let Some(weight) = args.get::<_, FontWeight>(ctx, "weight") {
        ctx.state.font.variant.weight = weight;
    }

    if let Some(stretch) = args.get::<_, FontStretch>(ctx, "stretch") {
        ctx.state.font.variant.stretch = stretch;
    }

    struct FamilyList(Vec<String>);

    try_from_match!(FamilyList["family or list of families"] @ span:
        Value::Str(v) => Self(vec![v.to_lowercase()]),
        Value::Dict(v) => Self(Args(v.with_span(span))
            .find_all::<StringLike>()
            .map(|s| s.to_lowercase())
            .collect()
        ),
    );

    for &class in &["serif", "sans-serif", "monospace", "emoji", "math"] {
        if let Some(list) = args.get::<_, FamilyList>(ctx, class) {
            Rc::make_mut(&mut ctx.state.font.families)
                .update_class_list(class.to_string(), list.0);
            needs_flattening = true;
        }
    }

    if needs_flattening {
        Rc::make_mut(&mut ctx.state.font.families).flatten();
    }

    args.done(ctx);

    if let Some(body) = body {
        body.eval(ctx);
        ctx.state = snapshot;
    }

    Value::None
}

/// `rgb`: Create an RGB(A) color.
///
/// # Positional arguments
/// - Red component (`float` between 0.0 and 1.0).
/// - Green component (`float` between 0.0 and 1.0).
/// - Blue component (`float` between 0.0 and 1.0).
/// - Alpha component (optional, `float` between 0.0 and 1.0).
pub fn rgb(mut args: Args, ctx: &mut EvalContext) -> Value {
    let r = args.need::<_, Spanned<f64>>(ctx, 0, "red component");
    let g = args.need::<_, Spanned<f64>>(ctx, 1, "green component");
    let b = args.need::<_, Spanned<f64>>(ctx, 2, "blue component");
    let a = args.get::<_, Spanned<f64>>(ctx, 3);
    args.done(ctx);

    let mut clamp = |component: Option<Spanned<f64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0.0 || c.v > 1.0 {
                ctx.diag(error!(c.span, "should be between 0.0 and 1.0"));
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

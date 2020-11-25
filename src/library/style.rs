use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontWeight};

use crate::color::RgbaColor;
use crate::eval::StringLike;
use crate::geom::Linear;
use crate::prelude::*;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - The font size (optional, length or relative to current font size).
/// - All identifier and string arguments are interpreted as an ordered list of
///   fallback font families.
///
/// An example invocation could look like this:
/// ```typst
/// [font: 12pt, Arial, "Noto Sans", sans-serif]
/// ```
///
/// # Keyword arguments
/// - `style`
///     - `normal`
///     - `italic`
///     - `oblique`
///
/// - `weight`
///     - `thin` or `hairline` (`100`)
///     - `extralight`         (`200`)
///     - `light`              (`300`)
///     - `regular`            (`400`)
///     - `medium`             (`500`)
///     - `semibold`           (`600`)
///     - `bold`               (`700`)
///     - `extrabold`          (`800`)
///     - `black`              (`900`)
///     - integer between `100` and `900`
///
/// - `stretch`
///     - `ultra-condensed`
///     - `extra-condensed`
///     - `condensed`
///     - `semi-condensed`
///     - `normal`
///     - `semi-expanded`
///     - `expanded`
///     - `extra-expanded`
///     - `ultra-expanded`
///
/// - Any other keyword argument whose value is a dictionary of strings defines
///   a fallback class, for example:
///   ```typst
///   [font: serif = ("Source Serif Pro", "Noto Serif")]
///   ```
///   This class can be used in the fallback list or other fallback classes as
///   long as the resulting fallback tree is acyclic.
///   ```typst
///   [font: "My Serif", serif]
///   ```
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

    for (class, dict) in args.find_all_str::<Spanned<ValueDict>>() {
        let fallback = Args(dict)
            .find_all::<StringLike>()
            .map(|s| s.to_lowercase())
            .collect();

        Rc::make_mut(&mut ctx.state.font.families).update_class_list(class, fallback);
        needs_flattening = true;
    }

    args.done(ctx);

    if needs_flattening {
        Rc::make_mut(&mut ctx.state.font.families).flatten();
    }

    if let Some(body) = body {
        body.eval(ctx);
        ctx.state = snapshot;
    }

    Value::None
}

/// `rgb`: Create an RGB(A) color.
///
/// # Positional arguments
/// - The red component (integer between 0 and 255).
/// - The green component (integer between 0 and 255).
/// - The blue component (integer between 0 and 255).
/// - The alpha component (optional, integer between 0 and 255).
pub fn rgb(mut args: Args, ctx: &mut EvalContext) -> Value {
    let r = args.need::<_, Spanned<i64>>(ctx, 0, "red value");
    let g = args.need::<_, Spanned<i64>>(ctx, 1, "green value");
    let b = args.need::<_, Spanned<i64>>(ctx, 2, "blue value");
    let a = args.get::<_, Spanned<i64>>(ctx, 3);
    args.done(ctx);

    let mut clamp = |component: Option<Spanned<i64>>, default| {
        component.map_or(default, |c| {
            if c.v < 0 || c.v > 255 {
                ctx.diag(error!(c.span, "should be between 0 and 255"));
            }
            c.v.max(0).min(255) as u8
        })
    };

    Value::Color(RgbaColor::new(
        clamp(r, 0),
        clamp(g, 0),
        clamp(b, 0),
        clamp(a, 255),
    ))
}

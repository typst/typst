use std::rc::Rc;

use fontdock::{FontStretch, FontStyle, FontWeight};

use crate::eval::StringLike;
use crate::geom::Linear;
use crate::prelude::*;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - The font size (optional, length or relative to previous font size).
/// - A font family fallback list (optional, identifiers or strings).
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
///     - any integer from the range `100` - `900` (inclusive)
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

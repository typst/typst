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
///   long as the resulting fallback tree is acylic.
///   ```typst
///   [font: "My Serif", serif]
///   ```
pub async fn font(mut args: Args, ctx: &mut LayoutContext) -> Value {
    let mut text = ctx.state.text.clone();
    let mut needs_flatten = false;

    let body = args.find::<SynTree>();

    if let Some(linear) = args.find::<Linear>() {
        if linear.rel == 0.0 {
            text.font_size.base = linear.abs;
            text.font_size.scale = Linear::rel(1.0);
        } else {
            text.font_size.scale = linear;
        }
    }

    let list: Vec<_> = args.find_all::<StringLike>().map(|s| s.to_lowercase()).collect();
    if !list.is_empty() {
        text.fallback.list = list;
        needs_flatten = true;
    }

    if let Some(style) = args.get::<_, FontStyle>(ctx, "style") {
        text.variant.style = style;
    }

    if let Some(weight) = args.get::<_, FontWeight>(ctx, "weight") {
        text.variant.weight = weight;
    }

    if let Some(stretch) = args.get::<_, FontStretch>(ctx, "stretch") {
        text.variant.stretch = stretch;
    }

    for (class, dict) in args.find_all_str::<Spanned<ValueDict>>() {
        let fallback = Args(dict)
            .find_all::<StringLike>()
            .map(|s| s.to_lowercase())
            .collect();

        text.fallback.update_class_list(class, fallback);
        needs_flatten = true;
    }

    args.done(ctx);

    if needs_flatten {
        text.fallback.flatten();
    }

    Value::Commands(match body {
        Some(tree) => vec![
            SetTextState(text),
            LayoutSyntaxTree(tree),
            SetTextState(ctx.state.text.clone()),
        ],
        None => vec![SetTextState(text)],
    })
}

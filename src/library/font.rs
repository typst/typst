use fontdock::{FontStretch, FontStyle, FontWeight};

use super::*;
use crate::eval::StringLike;
use crate::geom::Linear;

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
pub async fn font(mut args: ValueDict, ctx: &mut LayoutContext) -> Value {
    let mut text = ctx.state.text.clone();
    let mut updated_fallback = false;

    let content = args.take::<SynTree>();

    if let Some(linear) = args.take::<Linear>() {
        if linear.rel == 0.0 {
            text.font_size.base = linear.abs;
            text.font_size.scale = Linear::rel(1.0);
        } else {
            text.font_size.scale = linear;
        }
    }

    let list: Vec<_> = args
        .take_all_num_vals::<StringLike>()
        .map(|s| s.to_lowercase())
        .collect();

    if !list.is_empty() {
        text.fallback.list = list;
        updated_fallback = true;
    }

    if let Some(style) = args.take_key::<FontStyle>("style", &mut ctx.f) {
        text.variant.style = style;
    }

    if let Some(weight) = args.take_key::<FontWeight>("weight", &mut ctx.f) {
        text.variant.weight = weight;
    }

    if let Some(stretch) = args.take_key::<FontStretch>("stretch", &mut ctx.f) {
        text.variant.stretch = stretch;
    }

    for (class, mut dict) in args.take_all_str::<ValueDict>() {
        let fallback = dict
            .take_all_num_vals::<StringLike>()
            .map(|s| s.to_lowercase())
            .collect();

        text.fallback.update_class_list(class, fallback);
        updated_fallback = true;
    }

    if updated_fallback {
        text.fallback.flatten();
    }

    args.unexpected(&mut ctx.f);
    Value::Commands(match content {
        Some(tree) => vec![
            SetTextState(text),
            LayoutSyntaxTree(tree),
            SetTextState(ctx.state.text.clone()),
        ],
        None => vec![SetTextState(text)],
    })
}

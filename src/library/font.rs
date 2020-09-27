use fontdock::{FontStretch, FontStyle, FontWeight};

use super::*;
use crate::length::ScaleLength;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - The font size (optional, length or relative to previous font size).
/// - A font family fallback list (optional, identifiers or strings).
///
/// # Keyword arguments
/// - `style`: `normal`, `italic` or `oblique`.
/// - `weight`: `100` - `900` or a name like `thin`.
/// - `width`: `normal`, `condensed`, `expanded`, ...
/// - Any other keyword argument whose value is a table of strings is a class
///   fallback definition like:
///   ```typst
///   serif = ("Source Serif Pro", "Noto Serif")
///   ```
pub async fn font(_: Span, mut args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();
    let mut text = ctx.style.text.clone();
    let mut updated_fallback = false;

    let content = args.take::<SyntaxTree>();

    if let Some(s) = args.take::<ScaleLength>() {
        match s {
            ScaleLength::Absolute(length) => {
                text.base_font_size = length.as_raw();
                text.font_scale = 1.0;
            }
            ScaleLength::Scaled(scale) => text.font_scale = scale,
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

    if let Some(style) = args.take_key::<FontStyle>("style", &mut f) {
        text.variant.style = style;
    }

    if let Some(weight) = args.take_key::<FontWeight>("weight", &mut f) {
        text.variant.weight = weight;
    }

    if let Some(stretch) = args.take_key::<FontStretch>("stretch", &mut f) {
        text.variant.stretch = stretch;
    }

    for (class, mut table) in args.take_all_str::<TableValue>() {
        let fallback = table
            .take_all_num_vals::<StringLike>()
            .map(|s| s.to_lowercase())
            .collect();

        text.fallback.update_class_list(class, fallback);
        updated_fallback = true;
    }

    if updated_fallback {
        text.fallback.flatten();
    }

    let commands = match content {
        Some(tree) => vec![
            SetTextStyle(text),
            LayoutSyntaxTree(tree),
            SetTextStyle(ctx.style.text.clone()),
        ],
        None => vec![SetTextStyle(text)],
    };

    args.unexpected(&mut f);
    Pass::commands(commands, f)
}

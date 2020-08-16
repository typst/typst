use fontdock::{FontStyle, FontWeight, FontWidth};

use crate::length::ScaleLength;
use super::*;

/// `font`: Configure the font.
///
/// # Positional arguments
/// - The font size (optional, length or relative to previous font size).
/// - A font family fallback list (optional, identifiers or strings).
///
/// # Keyword arguments
/// - `style`: `normal`, `italic` or `oblique`.
/// - `weight`: `100` - `900` or a name like `thin`.
/// - `width`: `1` - `9` or a name like `condensed`.
/// - Any other keyword argument whose value is a table of strings is a class
///   fallback definition like:
///   ```typst
///   serif = ("Source Serif Pro", "Noto Serif")
///   ```
pub async fn font(mut args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();

    let content = args.take::<SyntaxTree>();
    let size = args.take::<ScaleLength>();
    let style = args.take_with_key::<_, FontStyle>("style", &mut f);
    let weight = args.take_with_key::<_, FontWeight>("weight", &mut f);
    let width = args.take_with_key::<_, FontWidth>("width", &mut f);
    let list: Vec<_> = args.take_all_num_vals::<StringLike>()
        .map(|s| s.0.to_lowercase())
        .collect();
    let classes: Vec<(_, Vec<_>)> = args.take_all_str::<TableValue>()
        .map(|(class, mut table)| {
            let fallback = table.take_all_num_vals::<StringLike>()
                .map(|s| s.0.to_lowercase())
                .collect();
            (class, fallback)
        })
        .collect();

    args.unexpected(&mut f);

    let mut text = ctx.style.text.clone();

    size.with(|s| match s {
        ScaleLength::Absolute(length) => {
            text.base_font_size = length.as_raw();
            text.font_scale = 1.0;
        }
        ScaleLength::Scaled(scale) => text.font_scale = scale,
    });

    style.with(|s| text.variant.style = s);
    weight.with(|w| text.variant.weight = w);
    width.with(|w| text.variant.width = w);

    if !list.is_empty() {
        *text.fallback.list_mut() = list.iter()
            .map(|s| s.to_lowercase())
            .collect();
    }

    for (class, fallback) in classes {
        text.fallback.set_class_list(class.clone(), fallback.clone());
    }

    text.fallback.flatten();

    Pass::commands(match content {
        Some(tree) => vec![
            SetTextStyle(text),
            LayoutSyntaxTree(tree),
            SetTextStyle(ctx.style.text.clone()),
        ],
        None => vec![SetTextStyle(text)],
    }, f)
}

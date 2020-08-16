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
pub fn font(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    let mut args = call.args;

    let node = FontNode {
        content: args.take::<SyntaxTree>(),
        size: args.take::<ScaleLength>(),
        style: args.take_with_key::<_, FontStyle>("style", &mut f),
        weight: args.take_with_key::<_, FontWeight>("weight", &mut f),
        width: args.take_with_key::<_, FontWidth>("width", &mut f),
        list: args.take_all_num_vals::<StringLike>()
            .map(|s| s.0.to_lowercase())
            .collect(),
        classes: args.take_all_str::<TableExpr>()
            .map(|(class, mut table)| {
                let fallback = table.take_all_num_vals::<StringLike>()
                    .map(|s| s.0.to_lowercase())
                    .collect();
                (class, fallback)
            })
            .collect()
    };

    args.unexpected(&mut f);
    Pass::node(node, f)
}

#[derive(Debug, Clone, PartialEq)]
struct FontNode {
    content: Option<SyntaxTree>,
    size: Option<ScaleLength>,
    style: Option<FontStyle>,
    weight: Option<FontWeight>,
    width: Option<FontWidth>,
    list: Vec<String>,
    classes: Vec<(String, Vec<String>)>,
}

#[async_trait(?Send)]
impl Layout for FontNode {
    async fn layout<'a>(&'a self, ctx: LayoutContext<'_>) -> Pass<Commands<'a>> {
        let mut text = ctx.style.text.clone();

        self.size.with(|s| match s {
            ScaleLength::Absolute(length) => {
                text.base_font_size = length.as_raw();
                text.font_scale = 1.0;
            }
            ScaleLength::Scaled(scale) => text.font_scale = scale,
        });

        self.style.with(|s| text.variant.style = s);
        self.weight.with(|w| text.variant.weight = w);
        self.width.with(|w| text.variant.width = w);

        if !self.list.is_empty() {
            *text.fallback.list_mut() = self.list.iter()
                .map(|s| s.to_lowercase())
                .collect();
        }

        for (class, fallback) in &self.classes {
            text.fallback.set_class_list(class.clone(), fallback.clone());
        }

        text.fallback.flatten();

        Pass::okay(match &self.content {
            Some(tree) => vec![
                SetTextStyle(text),
                LayoutSyntaxTree(tree),
                SetTextStyle(ctx.style.text.clone()),
            ],
            None => vec![SetTextStyle(text)],
        })
    }
}

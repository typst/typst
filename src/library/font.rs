use fontdock::{FontStyle, FontWeight, FontWidth};

use crate::length::ScaleLength;
use super::*;

function! {
    /// `font`: Configure the font.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontFunc {
        body: Option<SyntaxTree>,
        size: Option<ScaleLength>,
        style: Option<FontStyle>,
        weight: Option<FontWeight>,
        width: Option<FontWidth>,
        list: Vec<String>,
        classes: Vec<(String, Vec<String>)>,
    }

    parse(header, body, state, f) {
        let size = header.args.pos.get::<ScaleLength>();
        let style = header.args.key.get::<FontStyle>("style", f);
        let weight = header.args.key.get::<FontWeight>("weight", f);
        let width = header.args.key.get::<FontWidth>("width", f);

        let list = header.args.pos.all::<StringLike>()
            .map(|s| s.0.to_lowercase())
            .collect();

        let classes = header.args.key
            .all::<Tuple>()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(class, mut tuple)| {
                let fallback = tuple.all::<StringLike>()
                    .map(|s| s.0.to_lowercase())
                    .collect();
                (class.v.0, fallback)
            })
            .collect();

        Self {
            body: parse_maybe_body(body, state, f),
            size,
            style,
            weight,
            width,
            list,
            classes,
        }
    }

    layout(self, ctx, f) {
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

        match &self.body {
            Some(tree) => vec![
                SetTextStyle(text),
                LayoutSyntaxTree(tree),
                SetTextStyle(ctx.style.text.clone()),
            ],
            None => vec![SetTextStyle(text)],
        }
    }
}

use fontdock::{FontStyle, FontWeight, FontWidth};
use crate::length::ScaleLength;
use super::*;

function! {
    /// `font`: Configure the font.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontFunc {
        body: Option<SyntaxModel>,
        size: Option<ScaleLength>,
        style: Option<FontStyle>,
        weight: Option<FontWeight>,
        width: Option<FontWidth>,
        list: Vec<String>,
        classes: Vec<(String, Vec<String>)>,
    }

    parse(header, body, ctx, f) {
        let size = header.args.pos.get_first::<ScaleLength>(&mut f.diagnostics);

        let style = header.args.key.get::<FontStyle>(&mut f.diagnostics, "style");
        let weight = header.args.key.get::<FontWeight>(&mut f.diagnostics, "weight");
        let width = header.args.key.get::<FontWidth>(&mut f.diagnostics, "width");

        let list = header.args.pos.get_all::<StringLike>(&mut f.diagnostics)
            .map(|s| s.0.to_lowercase())
            .collect();

        let classes = header.args.key
            .get_all::<String, Tuple>(&mut f.diagnostics)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(class, mut tuple)| {
                let fallback = tuple.get_all::<StringLike>(&mut f.diagnostics)
                    .map(|s| s.0.to_lowercase())
                    .collect();
                (class.to_lowercase(), fallback)
            })
            .collect();

        FontFunc {
            body: body!(opt: body, ctx, f),
            size,
            list,
            classes,
            style,
            weight,
            width,
        }
    }

    layout(self, ctx, f) {
        styled(&self.body, ctx, Some(()),
            |t, _| {
                self.size.with(|s| match s {
                    ScaleLength::Absolute(length) => {
                        t.base_font_size = length.as_raw();
                        t.font_scale = 1.0;
                    }
                    ScaleLength::Scaled(scale) => t.font_scale = scale,
                });

                self.style.with(|s| t.variant.style = s);
                self.weight.with(|w| t.variant.weight = w);
                self.width.with(|w| t.variant.width = w);

                if !self.list.is_empty() {
                    *t.fallback.list_mut() = self.list.clone();
                }

                for (class, fallback) in &self.classes {
                    t.fallback.set_class_list(class.clone(), fallback.clone());
                }

                t.fallback.flatten();
            })
    }
}

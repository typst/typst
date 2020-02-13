use toddle::query::{FontWeight, FontStyle};
use crate::size::FSize;
use super::*;


function! {
    /// `font.family`: Set the font family.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontFamilyFunc {
        body: Option<SyntaxModel>,
        list: Vec<String>,
        classes: Vec<(String, Vec<String>)>,
    }

    parse(header, body, ctx, f) {
        let list = header.args.pos.get_all::<StringLike>(&mut f.errors)
            .map(|s| s.0.to_lowercase())
            .collect();

        let tuples: Vec<_> = header.args.key
            .get_all::<String, Tuple>(&mut f.errors)
            .collect();

        let classes = tuples.into_iter()
            .map(|(class, mut tuple)| {
                let fallback = tuple.get_all::<StringLike>(&mut f.errors)
                    .map(|s| s.0.to_lowercase())
                    .collect();
                (class.to_lowercase(), fallback)
            })
            .collect();

        FontFamilyFunc {
            body: body!(opt: body, ctx, f),
            list,
            classes,
        }
    }

    layout(self, ctx, errors) {
        styled(&self.body, ctx, Some(()),
            |s, _| {
                if !self.list.is_empty() {
                    s.fallback.list = self.list.clone();
                }

                for (class, fallback) in &self.classes {
                    s.fallback.set_class_list(class.clone(), fallback.clone());
                }

                s.fallback.flatten();
            })
    }
}

function! {
    /// `font.style`: Set the font style (normal / italic).
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontStyleFunc {
        body: Option<SyntaxModel>,
        style: Option<FontStyle>,
    }

    parse(header, body, ctx, f) {
        FontStyleFunc {
            body: body!(opt: body, ctx, f),
            style: header.args.pos.get::<FontStyle>(&mut f.errors)
                .or_missing(&mut f.errors, header.name.span, "style"),
        }
    }

    layout(self, ctx, errors) {
        styled(&self.body, ctx, self.style, |t, s| t.variant.style = s)
    }
}

function! {
    /// `font.weight`: Set text with a given weight.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontWeightFunc {
        body: Option<SyntaxModel>,
        weight: Option<FontWeight>,
    }

    parse(header, body, ctx, f) {
        let body = body!(opt: body, ctx, f);
        let weight = header.args.pos.get::<Spanned<(FontWeight, bool)>>(&mut f.errors)
            .map(|Spanned { v: (weight, is_clamped), span }| {
                if is_clamped {
                    f.errors.push(err!(@Warning: span;
                        "weight should be between \
                         100 and 900, clamped to {}", weight.0));
                }

                weight
            })
            .or_missing(&mut f.errors, header.name.span, "weight");

        FontWeightFunc { body, weight }
    }

    layout(self, ctx, errors) {
        styled(&self.body, ctx, self.weight, |t, w| t.variant.weight = w)
    }
}

function! {
    /// `font.size`: Sets the font size.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontSizeFunc {
        body: Option<SyntaxModel>,
        size: Option<FSize>,
    }

    parse(header, body, ctx, f) {
        FontSizeFunc {
            body: body!(opt: body, ctx, f),
            size: header.args.pos.get::<FSize>(&mut f.errors)
                .or_missing(&mut f.errors, header.name.span, "size")
        }
    }

    layout(self, ctx, errors) {
        styled(&self.body, ctx, self.size, |t, s| {
            match s {
                FSize::Absolute(size) => {
                    t.base_font_size = size;
                    t.font_scale = 1.0;
                }
                FSize::Scaled(scale) => t.font_scale = scale,
            }
        })
    }
}

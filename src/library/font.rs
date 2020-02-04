use toddle::query::{FontWeight, FontStyle};
use crate::size::FSize;
use super::*;


function! {
    /// `font.family`: Set the font family.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontFamilyFunc {
        body: Option<SyntaxModel>,
        list: Vec<String>,
    }

    parse(header, body, ctx, errors, decos) {
        FontFamilyFunc {
            body: body!(opt: body, ctx, errors, decos),
            list: header.args.pos.get_all::<StringLike>(errors)
                .map(|s| s.0.to_lowercase())
                .collect(),
        }
    }

    layout(self, ctx, errors) {
        styled(&self.body, ctx, Some(&self.list),
            |s, list| {
                s.fallback.list = list.clone();
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

    parse(header, body, ctx, errors, decos) {
        FontStyleFunc {
            body: body!(opt: body, ctx, errors, decos),
            style: header.args.pos.get::<FontStyle>(errors)
                .or_missing(errors, header.name.span, "style"),
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

    parse(header, body, ctx, errors, decos) {
        let body = body!(opt: body, ctx, errors, decos);
        let weight = header.args.pos.get::<Spanned<(FontWeight, bool)>>(errors)
            .map(|Spanned { v: (weight, is_clamped), span }| {
                if is_clamped {
                    errors.push(err!(@Warning: span;
                        "weight should be between \
                         100 and 900, clamped to {}", weight.0));
                }

                weight
            })
            .or_missing(errors, header.name.span, "weight");

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

    parse(header, body, ctx, errors, decos) {
        FontSizeFunc {
            body: body!(opt: body, ctx, errors, decos),
            size: header.args.pos.get::<FSize>(errors)
                .or_missing(errors, header.name.span, "size")
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

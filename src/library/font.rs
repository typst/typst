use fontdock::{FontStyle, FontWeight, FontWidth};
use crate::length::ScaleLength;
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
        let list = header.args.pos.get_all::<StringLike>(&mut f.diagnostics)
            .map(|s| s.0.to_lowercase())
            .collect();

        let tuples: Vec<_> = header.args.key
            .get_all::<String, Tuple>(&mut f.diagnostics)
            .collect();

        let classes = tuples.into_iter()
            .map(|(class, mut tuple)| {
                let fallback = tuple.get_all::<StringLike>(&mut f.diagnostics)
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

    layout(self, ctx, f) {
        styled(&self.body, ctx, Some(()),
            |s, _| {
                if !self.list.is_empty() {
                    *s.fallback.list_mut() = self.list.clone();
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
            style: header.args.pos.get::<FontStyle>(&mut f.diagnostics)
                .or_missing(&mut f.diagnostics, header.name.span, "style"),
        }
    }

    layout(self, ctx, f) {
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
        let weight = header.args.pos.get::<Spanned<(FontWeight, bool)>>(&mut f.diagnostics)
            .map(|Spanned { v: (weight, is_clamped), span }| {
                if is_clamped {
                    warning!(
                        @f, span,
                        "weight should be between 100 and 900, clamped to {}",
                        weight.0,
                    );
                }

                weight
            })
            .or_missing(&mut f.diagnostics, header.name.span, "weight");

        FontWeightFunc { body, weight }
    }

    layout(self, ctx, f) {
        styled(&self.body, ctx, self.weight, |t, w| t.variant.weight = w)
    }
}


function! {
    /// `font.width`: Set text with a given width.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontWidthFunc {
        body: Option<SyntaxModel>,
        width: Option<FontWidth>,
    }

    parse(header, body, ctx, f) {
        let body = body!(opt: body, ctx, f);
        let width = header.args.pos.get::<Spanned<(FontWidth, bool)>>(&mut f.diagnostics)
            .map(|Spanned { v: (width, is_clamped), span }| {
                if is_clamped {
                    warning!(
                        @f, span,
                        "width should be between 1 and 9, clamped to {}",
                        width.to_number(),
                    );
                }

                width
            })
            .or_missing(&mut f.diagnostics, header.name.span, "width");

            FontWidthFunc { body, width }
    }

    layout(self, ctx, f) {
        styled(&self.body, ctx, self.width, |t, w| t.variant.width = w)
    }
}

function! {
    /// `font.size`: Sets the font size.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FontSizeFunc {
        body: Option<SyntaxModel>,
        size: Option<ScaleLength>,
    }

    parse(header, body, ctx, f) {
        FontSizeFunc {
            body: body!(opt: body, ctx, f),
            size: header.args.pos.get::<ScaleLength>(&mut f.diagnostics)
                .or_missing(&mut f.diagnostics, header.name.span, "size")
        }
    }

    layout(self, ctx, f) {
        styled(&self.body, ctx, self.size, |t, s| {
            match s {
                ScaleLength::Absolute(length) => {
                    t.base_font_size = length.as_raw();
                    t.font_scale = 1.0;
                }
                ScaleLength::Scaled(scale) => t.font_scale = scale,
            }
        })
    }
}

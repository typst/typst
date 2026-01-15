use az::SaturatingAs;
use comemo::Tracked;
use rustybuzz::{BufferFlags, UnicodeBuffer};
use typst_library::World;
use typst_library::layout::Em;
use typst_library::text::{Font, FontFamily, FontVariant, Glyph};
use typst_syntax::Span;

use crate::inline::{SharedShapingContext, create_shape_plan, get_font_and_covers};

/// Shape some text in math.
#[comemo::memoize]
pub fn shape(
    world: Tracked<dyn World + '_>,
    variant: FontVariant,
    features: Vec<rustybuzz::Feature>,
    language: rustybuzz::Language,
    fallback: bool,
    text: &str,
    families: Vec<&FontFamily>,
) -> Option<(Font, Vec<Glyph>)> {
    let mut ctx = ShapingContext {
        world,
        used: vec![],
        variant,
        features,
        language,
        fallback,
        glyphs: vec![],
        font: None,
    };

    shape_impl(&mut ctx, text, families.into_iter());

    Some((ctx.font?, ctx.glyphs))
}

/// Holds shaping results and metadata for shaping some text.
struct ShapingContext<'a> {
    world: Tracked<'a, dyn World + 'a>,
    used: Vec<Font>,
    variant: FontVariant,
    features: Vec<rustybuzz::Feature>,
    language: rustybuzz::Language,
    fallback: bool,
    glyphs: Vec<Glyph>,
    font: Option<Font>,
}

impl<'a> SharedShapingContext<'a> for ShapingContext<'a> {
    fn world(&self) -> Tracked<'a, dyn World + 'a> {
        self.world
    }

    fn used(&mut self) -> &mut Vec<Font> {
        &mut self.used
    }

    fn first(&self) -> Option<&Font> {
        self.used.first()
    }

    fn variant(&self) -> FontVariant {
        self.variant
    }

    fn fallback(&self) -> bool {
        self.fallback
    }
}

/// Shape text with font fallback using the `families` iterator.
fn shape_impl<'a>(
    ctx: &mut ShapingContext<'a>,
    text: &str,
    mut families: impl Iterator<Item = &'a FontFamily> + Clone,
) {
    let Some((font, covers)) =
        get_font_and_covers(ctx, text, families.by_ref(), |ctx, text, font| {
            let add_glyph = |_| {
                ctx.glyphs.push(Glyph {
                    id: 0,
                    x_advance: font.x_advance(0).unwrap_or_default(),
                    x_offset: Em::zero(),
                    y_advance: Em::zero(),
                    y_offset: Em::zero(),
                    range: 0..text.len().saturating_as(),
                    span: (Span::detached(), 0),
                })
            };
            text.chars().for_each(add_glyph);
            ctx.font = Some(font);
        })
    else {
        return;
    };

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_language(ctx.language.clone());
    // TODO: Use `rustybuzz::script::MATH` once
    // https://github.com/harfbuzz/rustybuzz/pull/165 is released.
    buffer.set_script(
        rustybuzz::Script::from_iso15924_tag(ttf_parser::Tag::from_bytes(b"math"))
            .unwrap(),
    );
    buffer.set_direction(rustybuzz::Direction::LeftToRight);
    buffer.set_flags(BufferFlags::REMOVE_DEFAULT_IGNORABLES);

    let plan = create_shape_plan(
        &font,
        buffer.direction(),
        buffer.script(),
        buffer.language().as_ref(),
        &ctx.features,
    );

    let buffer = rustybuzz::shape_with_plan(font.rusty(), &plan, buffer);
    // Because we will only ever shape single grapheme clusters, we will
    // (incorrectly) assume that the output from the shaper is a single cluster
    // that spans the entire range of the given text. The only problem this
    // could cause is the ranges for glyphs being incorrect in the final
    // `TextItem`, which could then affect text extraction in PDF export.

    if buffer.glyph_infos().iter().any(|i| i.glyph_id == 0)
        || !covers.is_none_or(|cov| cov.is_match(text))
    {
        shape_impl(ctx, text, families);
    } else {
        for i in 0..buffer.len() {
            let info = buffer.glyph_infos()[i];
            let pos = buffer.glyph_positions()[i];
            ctx.glyphs.push(Glyph {
                id: info.glyph_id as u16,
                x_advance: font.to_em(pos.x_advance),
                x_offset: font.to_em(pos.x_offset),
                y_advance: font.to_em(pos.y_advance),
                y_offset: font.to_em(pos.y_offset),
                range: 0..text.len().saturating_as(),
                span: (Span::detached(), 0),
            });
        }
        if !buffer.is_empty() {
            ctx.font = Some(font);
        }
    }
}

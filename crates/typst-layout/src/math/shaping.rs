use az::SaturatingAs;
use comemo::Tracked;
use rustybuzz::{
    BufferFlags, Direction, Feature, Language, Script, UnicodeBuffer, shape_with_plan,
};
use ttf_parser::Tag;
use typst_library::World;
use typst_library::foundations::StyleChain;
use typst_library::layout::Em;
use typst_library::math::families;
use typst_library::text::{
    Font, FontFamily, FontVariant, Glyph, TextElem, language, variant,
};
use typst_syntax::Span;

use crate::inline::{SharedShapingContext, create_shape_plan, get_font_and_covers};

/// Shape some text in math.
pub fn shape(
    world: Tracked<dyn World + '_>,
    styles: StyleChain,
    features: &[Feature],
    text: &str,
) -> Option<(Font, Vec<Glyph>)> {
    shape_impl(
        world,
        variant(styles),
        features,
        language(styles),
        styles.get(TextElem::fallback),
        text,
        families(styles).collect(),
    )
}

/// Internal shaping implementation.
fn shape_impl(
    world: Tracked<dyn World + '_>,
    variant: FontVariant,
    features: &[Feature],
    language: Language,
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

    shape_text(&mut ctx, text, families.into_iter());

    Some((ctx.font?, ctx.glyphs))
}

/// Calls `retry` with feature slices that turn `flac`/`ssty` on or off in
/// different combinations until the closure returns `true`.
pub fn fallback<F>(mut features: Vec<Feature>, mut retry: F)
where
    F: FnMut(&[Feature]) -> bool,
{
    const FLAC: Tag = Tag::from_bytes(b"flac");
    const SSTY: Tag = Tag::from_bytes(b"ssty");

    // (flac, ssty) combinations to try.
    const OPTIONS: [(bool, u32); 6] =
        [(true, 2), (true, 1), (true, 0), (false, 2), (false, 1), (false, 0)];

    let had_flac = features
        .iter()
        .rev()
        .find(|f| f.tag == FLAC)
        .is_some_and(|f| f.value != 0);
    let had_ssty = features
        .iter()
        .rev()
        .find(|f| f.tag == SSTY)
        .map(|f| f.value)
        .unwrap_or(0);

    features.retain(|f| f.tag != FLAC && f.tag != SSTY);
    let base_len = features.len();

    for (flac, ssty) in OPTIONS {
        // Don't enable a feature the caller didn't pass in.
        if (flac && !had_flac) || ssty > had_ssty {
            continue;
        }
        // Don't retry the original.
        if (flac, ssty) == (had_flac, had_ssty) {
            continue;
        }

        features.truncate(base_len);
        if flac {
            features.push(Feature::new(FLAC, 1, ..));
        }
        if ssty > 0 {
            features.push(Feature::new(SSTY, ssty, ..));
        }

        if retry(&features) {
            break;
        }
    }
}

/// Holds shaping results and metadata for shaping some text.
struct ShapingContext<'a, 'b> {
    world: Tracked<'a, dyn World + 'a>,
    used: Vec<Font>,
    variant: FontVariant,
    features: &'b [Feature],
    language: Language,
    fallback: bool,
    glyphs: Vec<Glyph>,
    font: Option<Font>,
}

impl<'a, 'b> SharedShapingContext<'a> for ShapingContext<'a, 'b> {
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
fn shape_text<'a, 'b>(
    ctx: &mut ShapingContext<'a, 'b>,
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
    buffer.set_script(Script::from_iso15924_tag(Tag::from_bytes(b"math")).unwrap());
    buffer.set_direction(Direction::LeftToRight);
    buffer.set_flags(BufferFlags::REMOVE_DEFAULT_IGNORABLES);

    let plan = create_shape_plan(
        &font,
        buffer.direction(),
        buffer.script(),
        buffer.language().as_ref(),
        ctx.features,
    );

    let buffer = shape_with_plan(font.rusty(), &plan, buffer);
    // Because we will only ever shape single grapheme clusters, we will
    // (incorrectly) assume that the output from the shaper is a single cluster
    // that spans the entire range of the given text. The only problem this
    // could cause is the ranges for glyphs being incorrect in the final
    // `TextItem`, which could then affect text extraction in PDF export.

    if buffer.glyph_infos().iter().any(|i| i.glyph_id == 0)
        || !covers.is_none_or(|cov| cov.is_match(text))
    {
        shape_text(ctx, text, families);
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

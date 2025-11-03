use std::ops::{Deref, DerefMut};

use az::SaturatingAs;
use comemo::Tracked;
use ecow::EcoString;
use rustybuzz::{BufferFlags, UnicodeBuffer};
use typst_library::World;
use typst_library::layout::{Abs, Em};
use typst_library::text::{Font, FontFamily, FontVariant, Glyph, Lang, Region, TextItem};
use typst_library::visualize::{FixedStroke, Paint};
use typst_syntax::Span;

use crate::inline::{SharedShapingContext, create_shape_plan, get_font_and_covers};

/// A text item in Math.
///
/// This type is almost identical to
/// [`TextItem`](typst_library::text::TextItem), the difference being the
/// representation of the glyphs. See [`Glyphs`] for more info.
#[derive(Clone)]
pub struct ShapedText {
    /// The text that was shaped.
    pub text: EcoString,
    /// The text's font.
    pub font: Font,
    /// The text's size
    pub size: Abs,
    /// Glyph color.
    pub fill: Paint,
    /// Glyph stroke.
    pub stroke: Option<FixedStroke>,
    /// The natural language of the text.
    pub lang: Lang,
    /// The region of the text.
    pub region: Option<Region>,
    /// Should the text be selectable.
    pub selectable: bool,
    /// The text's span.
    pub span: Span,
    /// The shaped glyphs.
    pub glyphs: Glyphs,
}

impl ShapedText {
    /// The width of the text run.
    pub fn width(&self) -> Abs {
        self.glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(self.size)
    }

    /// The id of the first glyph in the original text.
    pub fn original_id(&self) -> u16 {
        self.glyphs.original()[0].id
    }
}

impl From<ShapedText> for TextItem {
    fn from(item: ShapedText) -> Self {
        TextItem {
            font: item.font,
            size: item.size,
            fill: item.fill,
            stroke: item.stroke,
            lang: item.lang,
            region: item.region,
            selectable: item.selectable,
            text: item.text.clone(),
            glyphs: item
                .glyphs
                .iter()
                .map(|g| Glyph {
                    id: g.id,
                    x_advance: g.x_advance,
                    x_offset: g.x_offset,
                    y_advance: g.y_advance,
                    y_offset: g.y_offset,
                    range: 0..item.text.len().saturating_as(),
                    span: (item.span, 0),
                })
                .collect(),
        }
    }
}

/// A collection of glyphs that stores the original set of glyphs when created.
#[derive(Clone)]
pub struct Glyphs {
    original: Vec<ShapedGlyph>,
    updated: Option<Vec<ShapedGlyph>>,
}

impl Deref for Glyphs {
    type Target = Vec<ShapedGlyph>;

    fn deref(&self) -> &Self::Target {
        self.updated.as_ref().unwrap_or(&self.original)
    }
}

impl DerefMut for Glyphs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.updated.as_mut().unwrap_or(self.original.as_mut())
    }
}

impl Glyphs {
    /// Create a new set of glyphs.
    pub fn new(initial: Vec<ShapedGlyph>) -> Self {
        Self { original: initial, updated: None }
    }

    /// Update the glyphs with the given value.
    pub fn update(&mut self, new_value: Vec<ShapedGlyph>) {
        self.updated = Some(new_value);
    }

    /// Reset the glyphs back their originals.
    pub fn reset(&mut self) {
        self.updated = None;
    }

    fn original(&self) -> &[ShapedGlyph] {
        &self.original
    }
}

/// A single glyph resulting from shaping.
#[derive(Clone)]
pub struct ShapedGlyph {
    /// The glyph's index in the font.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The advance height of the glyph.
    pub y_advance: Em,
    /// The vertical offset of the glyph.
    pub y_offset: Em,
}

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
) -> Option<(Font, Vec<ShapedGlyph>)> {
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
    glyphs: Vec<ShapedGlyph>,
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
                ctx.glyphs.push(ShapedGlyph {
                    id: 0,
                    x_advance: font.x_advance(0).unwrap_or_default(),
                    x_offset: Em::zero(),
                    y_advance: Em::zero(),
                    y_offset: Em::zero(),
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
            ctx.glyphs.push(ShapedGlyph {
                id: info.glyph_id as u16,
                x_advance: font.to_em(pos.x_advance),
                x_offset: font.to_em(pos.x_offset),
                y_advance: font.to_em(pos.y_advance),
                y_offset: font.to_em(pos.y_offset),
            });
        }
        if !buffer.is_empty() {
            ctx.font = Some(font);
        }
    }
}

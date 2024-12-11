use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::str::FromStr;
use std::sync::Arc;

use az::SaturatingAs;
use ecow::EcoString;
use rustybuzz::{BufferFlags, ShapePlan, UnicodeBuffer};
use ttf_parser::Tag;
use typst_library::engine::Engine;
use typst_library::foundations::{Smart, StyleChain};
use typst_library::layout::{Abs, Dir, Em, Frame, FrameItem, Point, Size};
use typst_library::text::{
    families, features, is_default_ignorable, variant, Font, FontFamily, FontVariant,
    Glyph, Lang, Region, TextEdgeBounds, TextElem, TextItem,
};
use typst_library::World;
use typst_utils::SliceExt;
use unicode_bidi::{BidiInfo, Level as BidiLevel};
use unicode_script::{Script, UnicodeScript};

use super::{decorate, Item, Range, SpanMapper};

/// The result of shaping text.
///
/// This type contains owned or borrowed shaped text runs, which can be
/// measured, used to reshape substrings more quickly and converted into a
/// frame.
#[derive(Clone)]
pub struct ShapedText<'a> {
    /// The start of the text in the full paragraph.
    pub base: usize,
    /// The text that was shaped.
    pub text: &'a str,
    /// The text direction.
    pub dir: Dir,
    /// The text language.
    pub lang: Lang,
    /// The text region.
    pub region: Option<Region>,
    /// The text's style properties.
    pub styles: StyleChain<'a>,
    /// The font variant.
    pub variant: FontVariant,
    /// The font size.
    pub size: Abs,
    /// The width of the text's bounding box.
    pub width: Abs,
    /// The shaped glyphs.
    pub glyphs: Cow<'a, [ShapedGlyph]>,
}

/// A single glyph resulting from shaping.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    /// The font the glyph is contained in.
    pub font: Font,
    /// The glyph's index in the font.
    pub glyph_id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The vertical offset of the glyph.
    pub y_offset: Em,
    /// The adjustability of the glyph.
    pub adjustability: Adjustability,
    /// The byte range of this glyph's cluster in the full paragraph. A cluster
    /// is a sequence of one or multiple glyphs that cannot be separated and
    /// must always be treated as a union.
    ///
    /// The range values of the glyphs in a [`ShapedText`] should not overlap
    /// with each other, and they should be monotonically increasing (for
    /// left-to-right or top-to-bottom text) or monotonically decreasing (for
    /// right-to-left or bottom-to-top text).
    pub range: Range,
    /// Whether splitting the shaping result before this glyph would yield the
    /// same results as shaping the parts to both sides of `text_index`
    /// separately.
    pub safe_to_break: bool,
    /// The first char in this glyph's cluster.
    pub c: char,
    /// Whether this glyph is justifiable for CJK scripts.
    pub is_justifiable: bool,
    /// The script of the glyph.
    pub script: Script,
}

#[derive(Debug, Clone, Default)]
pub struct Adjustability {
    /// The left and right stretchability
    pub stretchability: (Em, Em),
    /// The left and right shrinkability
    pub shrinkability: (Em, Em),
}

impl ShapedGlyph {
    /// Whether the glyph is a space.
    pub fn is_space(&self) -> bool {
        is_space(self.c)
    }

    /// Whether the glyph is justifiable.
    pub fn is_justifiable(&self) -> bool {
        // GB style is not relevant here.
        self.is_justifiable
    }

    /// Whether the glyph is part of Chinese or Japanese script (i.e. CJ, not CJK).
    pub fn is_cj_script(&self) -> bool {
        is_cj_script(self.c, self.script)
    }

    pub fn is_cjk_punctuation(&self) -> bool {
        self.is_cjk_left_aligned_punctuation(CjkPunctStyle::Gb)
            || self.is_cjk_right_aligned_punctuation()
            || self.is_cjk_center_aligned_punctuation(CjkPunctStyle::Gb)
    }

    /// See <https://www.w3.org/TR/clreq/#punctuation_width_adjustment>
    pub fn is_cjk_left_aligned_punctuation(&self, style: CjkPunctStyle) -> bool {
        is_cjk_left_aligned_punctuation(
            self.c,
            self.x_advance,
            self.stretchability(),
            style,
        )
    }

    /// See <https://www.w3.org/TR/clreq/#punctuation_width_adjustment>
    pub fn is_cjk_right_aligned_punctuation(&self) -> bool {
        is_cjk_right_aligned_punctuation(self.c, self.x_advance, self.stretchability())
    }

    /// See <https://www.w3.org/TR/clreq/#punctuation_width_adjustment>
    pub fn is_cjk_center_aligned_punctuation(&self, style: CjkPunctStyle) -> bool {
        is_cjk_center_aligned_punctuation(self.c, style)
    }

    /// Whether the glyph is a western letter or number.
    pub fn is_letter_or_number(&self) -> bool {
        matches!(self.c.script(), Script::Latin | Script::Greek | Script::Cyrillic)
            || matches!(self.c, '#' | '$' | '%' | '&')
            || self.c.is_ascii_digit()
    }

    pub fn base_adjustability(&self, style: CjkPunctStyle) -> Adjustability {
        let width = self.x_advance;
        if self.is_space() {
            Adjustability {
                // The number for spaces is from Knuth-Plass' paper
                stretchability: (Em::zero(), width / 2.0),
                shrinkability: (Em::zero(), width / 3.0),
            }
        } else if self.is_cjk_left_aligned_punctuation(style) {
            Adjustability {
                stretchability: (Em::zero(), Em::zero()),
                shrinkability: (Em::zero(), width / 2.0),
            }
        } else if self.is_cjk_right_aligned_punctuation() {
            Adjustability {
                stretchability: (Em::zero(), Em::zero()),
                shrinkability: (width / 2.0, Em::zero()),
            }
        } else if self.is_cjk_center_aligned_punctuation(style) {
            Adjustability {
                stretchability: (Em::zero(), Em::zero()),
                shrinkability: (width / 4.0, width / 4.0),
            }
        } else {
            Adjustability::default()
        }
    }

    /// The stretchability of the character.
    pub fn stretchability(&self) -> (Em, Em) {
        self.adjustability.stretchability
    }

    /// The shrinkability of the character.
    pub fn shrinkability(&self) -> (Em, Em) {
        self.adjustability.shrinkability
    }

    /// Shrink the width of glyph on the left side.
    pub fn shrink_left(&mut self, amount: Em) {
        self.x_offset -= amount;
        self.x_advance -= amount;
        self.adjustability.shrinkability.0 -= amount;
    }

    /// Shrink the width of glyph on the right side.
    pub fn shrink_right(&mut self, amount: Em) {
        self.x_advance -= amount;
        self.adjustability.shrinkability.1 -= amount;
    }
}

/// A side you can go toward.
enum Side {
    /// To the left-hand side.
    Left,
    /// To the right-hand side.
    Right,
}

impl<'a> ShapedText<'a> {
    /// Build the shaped text's frame.
    ///
    /// The `justification` defines how much extra advance width each
    /// [justifiable glyph](ShapedGlyph::is_justifiable) will get.
    pub fn build(
        &self,
        engine: &Engine,
        spans: &SpanMapper,
        justification_ratio: f64,
        extra_justification: Abs,
    ) -> Frame {
        let (top, bottom) = self.measure(engine);
        let size = Size::new(self.width, top + bottom);

        let mut offset = Abs::zero();
        let mut frame = Frame::soft(size);
        frame.set_baseline(top);

        let shift = TextElem::baseline_in(self.styles);
        let decos = TextElem::deco_in(self.styles);
        let fill = TextElem::fill_in(self.styles);
        let stroke = TextElem::stroke_in(self.styles);
        let span_offset = TextElem::span_offset_in(self.styles);

        for ((font, y_offset), group) in
            self.glyphs.as_ref().group_by_key(|g| (g.font.clone(), g.y_offset))
        {
            let mut range = group[0].range.clone();
            for glyph in group {
                range.start = range.start.min(glyph.range.start);
                range.end = range.end.max(glyph.range.end);
            }

            let pos = Point::new(offset, top + shift - y_offset.at(self.size));
            let glyphs: Vec<Glyph> = group
                .iter()
                .map(|shaped: &ShapedGlyph| {
                    let adjustability_left = if justification_ratio < 0.0 {
                        shaped.shrinkability().0
                    } else {
                        shaped.stretchability().0
                    };
                    let adjustability_right = if justification_ratio < 0.0 {
                        shaped.shrinkability().1
                    } else {
                        shaped.stretchability().1
                    };

                    let justification_left = adjustability_left * justification_ratio;
                    let mut justification_right =
                        adjustability_right * justification_ratio;
                    if shaped.is_justifiable() {
                        justification_right +=
                            Em::from_length(extra_justification, self.size)
                    }

                    frame.size_mut().x += justification_left.at(self.size)
                        + justification_right.at(self.size);

                    // We may not be able to reach the offset completely if
                    // it exceeds u16, but better to have a roughly correct
                    // span offset than nothing.
                    let mut span = spans.span_at(shaped.range.start);
                    span.1 = span.1.saturating_add(span_offset.saturating_as());

                    // |<---- a Glyph ---->|
                    //  -->|ShapedGlyph|<--
                    // +---+-----------+---+
                    // |   |  *********|   |
                    // |   |  *        |   |
                    // |   |  *    ****|   |
                    // |   |  *       *|   |
                    // |   |  *********|   |
                    // +---+--+--------+---+
                    //   A   B     C     D
                    // Note A, B, D could be positive, zero, or negative.
                    // A: justification_left
                    // B: ShapedGlyph's x_offset
                    //    (though a small part of the glyph may go inside B)
                    // B+C: ShapedGlyph's x_advance
                    // D: justification_right
                    // A+B: Glyph's x_offset
                    // A+B+C+D: Glyph's x_advance
                    Glyph {
                        id: shaped.glyph_id,
                        x_advance: shaped.x_advance
                            + justification_left
                            + justification_right,
                        x_offset: shaped.x_offset + justification_left,
                        range: (shaped.range.start - range.start).saturating_as()
                            ..(shaped.range.end - range.start).saturating_as(),
                        span,
                    }
                })
                .collect();

            let item = TextItem {
                font,
                size: self.size,
                lang: self.lang,
                region: self.region,
                fill: fill.clone(),
                stroke: stroke.clone().map(|s| s.unwrap_or_default()),
                text: self.text[range.start - self.base..range.end - self.base].into(),
                glyphs,
            };

            let width = item.width();
            if decos.is_empty() {
                frame.push(pos, FrameItem::Text(item));
            } else {
                // Apply line decorations.
                frame.push(pos, FrameItem::Text(item.clone()));
                for deco in &decos {
                    decorate(&mut frame, deco, &item, width, shift, pos);
                }
            }

            offset += width;
        }

        frame
    }

    /// Measure the top and bottom extent of this text.
    pub fn measure(&self, engine: &Engine) -> (Abs, Abs) {
        let mut top = Abs::zero();
        let mut bottom = Abs::zero();

        let top_edge = TextElem::top_edge_in(self.styles);
        let bottom_edge = TextElem::bottom_edge_in(self.styles);

        // Expand top and bottom by reading the font's vertical metrics.
        let mut expand = |font: &Font, bounds: TextEdgeBounds| {
            let (t, b) = font.edges(top_edge, bottom_edge, self.size, bounds);
            top.set_max(t);
            bottom.set_max(b);
        };

        if self.glyphs.is_empty() {
            // When there are no glyphs, we just use the vertical metrics of the
            // first available font.
            let world = engine.world;
            for family in families(self.styles) {
                if let Some(font) = world
                    .book()
                    .select(family.as_str(), self.variant)
                    .and_then(|id| world.font(id))
                {
                    expand(&font, TextEdgeBounds::Zero);
                    break;
                }
            }
        } else {
            for g in self.glyphs.iter() {
                expand(&g.font, TextEdgeBounds::Glyph(g.glyph_id));
            }
        }

        (top, bottom)
    }

    /// How many glyphs are in the text where we can insert additional
    /// space when encountering underfull lines.
    pub fn justifiables(&self) -> usize {
        self.glyphs.iter().filter(|g| g.is_justifiable()).count()
    }

    /// Whether the last glyph is a CJK character which should not be justified
    /// on line end.
    pub fn cjk_justifiable_at_last(&self) -> bool {
        self.glyphs
            .last()
            .map(|g| g.is_cj_script() || g.is_cjk_punctuation())
            .unwrap_or(false)
    }

    /// The stretchability of the text.
    pub fn stretchability(&self) -> Abs {
        self.glyphs
            .iter()
            .map(|g| g.stretchability().0 + g.stretchability().1)
            .sum::<Em>()
            .at(self.size)
    }

    /// The shrinkability of the text
    pub fn shrinkability(&self) -> Abs {
        self.glyphs
            .iter()
            .map(|g| g.shrinkability().0 + g.shrinkability().1)
            .sum::<Em>()
            .at(self.size)
    }

    /// Reshape a range of the shaped text, reusing information from this
    /// shaping process if possible.
    ///
    /// The text `range` is relative to the whole paragraph.
    pub fn reshape(&'a self, engine: &Engine, text_range: Range) -> ShapedText<'a> {
        let text = &self.text[text_range.start - self.base..text_range.end - self.base];
        if let Some(glyphs) = self.slice_safe_to_break(text_range.clone()) {
            #[cfg(debug_assertions)]
            assert_all_glyphs_in_range(glyphs, text, text_range.clone());
            Self {
                base: text_range.start,
                text,
                dir: self.dir,
                lang: self.lang,
                region: self.region,
                styles: self.styles,
                size: self.size,
                variant: self.variant,
                width: glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(self.size),
                glyphs: Cow::Borrowed(glyphs),
            }
        } else {
            shape(
                engine,
                text_range.start,
                text,
                self.styles,
                self.dir,
                self.lang,
                self.region,
            )
        }
    }

    /// Derive an empty text run with the same properties as this one.
    pub fn empty(&self) -> Self {
        Self {
            text: "",
            width: Abs::zero(),
            glyphs: Cow::Borrowed(&[]),
            ..*self
        }
    }

    /// Push a hyphen to end of the text.
    pub fn push_hyphen(&mut self, engine: &Engine, fallback: bool) {
        self.insert_hyphen(engine, fallback, Side::Right)
    }

    /// Prepend a hyphen to start of the text.
    pub fn prepend_hyphen(&mut self, engine: &Engine, fallback: bool) {
        self.insert_hyphen(engine, fallback, Side::Left)
    }

    fn insert_hyphen(&mut self, engine: &Engine, fallback: bool, side: Side) {
        let world = engine.world;
        let book = world.book();
        let fallback_func = if fallback {
            Some(|| book.select_fallback(None, self.variant, "-"))
        } else {
            None
        };
        let mut chain = families(self.styles)
            .map(|family| {
                family
                    .coverage()
                    .map_or(true, |c| c.is_match("-"))
                    .then(|| book.select(family.as_str(), self.variant))
                    .flatten()
            })
            .chain(fallback_func.iter().map(|f| f()))
            .flatten();

        chain.find_map(|id| {
            let font = world.font(id)?;
            let ttf = font.ttf();
            let glyph_id = ttf.glyph_index('-')?;
            let x_advance = font.to_em(ttf.glyph_hor_advance(glyph_id)?);
            let range = match side {
                Side::Left => self.glyphs.first().map(|g| g.range.start..g.range.start),
                Side::Right => self.glyphs.last().map(|g| g.range.end..g.range.end),
            }
            // In the unlikely chance that we hyphenate after an empty line,
            // ensure that the glyph range still falls after self.base so
            // that subtracting either of the endpoints by self.base doesn't
            // underflow. See <https://github.com/typst/typst/issues/2283>.
            .unwrap_or_else(|| self.base..self.base);
            self.width += x_advance.at(self.size);
            let glyph = ShapedGlyph {
                font,
                glyph_id: glyph_id.0,
                x_advance,
                x_offset: Em::zero(),
                y_offset: Em::zero(),
                adjustability: Adjustability::default(),
                range,
                safe_to_break: true,
                c: '-',
                is_justifiable: false,
                script: Script::Common,
            };
            match side {
                Side::Left => self.glyphs.to_mut().insert(0, glyph),
                Side::Right => self.glyphs.to_mut().push(glyph),
            }
            Some(())
        });
    }

    /// Find the subslice of glyphs that represent the given text range if both
    /// sides are safe to break.
    fn slice_safe_to_break(&self, text_range: Range) -> Option<&[ShapedGlyph]> {
        let Range { mut start, mut end } = text_range;
        if !self.dir.is_positive() {
            std::mem::swap(&mut start, &mut end);
        }

        let left = self.find_safe_to_break(start)?;
        let right = self.find_safe_to_break(end)?;
        Some(&self.glyphs[left..right])
    }

    /// Find the glyph offset matching the text index that is most towards the
    /// start of the text and safe-to-break.
    fn find_safe_to_break(&self, text_index: usize) -> Option<usize> {
        let ltr = self.dir.is_positive();

        // Handle edge cases.
        let len = self.glyphs.len();
        if text_index == self.base {
            return Some(if ltr { 0 } else { len });
        } else if text_index == self.base + self.text.len() {
            return Some(if ltr { len } else { 0 });
        }

        // Find any glyph with the text index.
        let found = self.glyphs.binary_search_by(|g: &ShapedGlyph| {
            let ordering = g.range.start.cmp(&text_index);
            if ltr {
                ordering
            } else {
                ordering.reverse()
            }
        });

        let mut idx = match found {
            Ok(idx) => idx,
            Err(idx) => {
                // Handle the special case where we break before a '\n'
                //
                // For example: (assume `a` is a CJK character with three bytes)
                // text:  " a     \n b  "
                // index:   0 1 2 3  4 5
                // text_index:    ^
                // glyphs:  0     .  1
                //
                // We will get found = Err(1), because '\n' does not have a
                // glyph. But it's safe to break here. Thus the following
                // condition:
                // - glyphs[0].end == text_index == 3
                // - text[3] == '\n'
                return (idx > 0
                    && self.glyphs[idx - 1].range.end == text_index
                    && self.text[text_index - self.base..].starts_with('\n'))
                .then_some(idx);
            }
        };

        // Search for the start-most glyph with the text index. This means
        // we take empty range glyphs at the start and leave those at the end
        // for the next line.
        let dec = if ltr { usize::checked_sub } else { usize::checked_add };
        while let Some(next) = dec(idx, 1) {
            if self.glyphs.get(next).map_or(true, |g| g.range.start != text_index) {
                break;
            }
            idx = next;
        }

        // RTL needs offset one because the left side of the range should be
        // exclusive and the right side inclusive, contrary to the normal
        // behaviour of ranges.
        self.glyphs[idx].safe_to_break.then_some(idx + usize::from(!ltr))
    }
}

impl Debug for ShapedText<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.text.fmt(f)
    }
}

/// Group a range of text by BiDi level and script, shape the runs and generate
/// items for them.
pub fn shape_range<'a>(
    items: &mut Vec<(Range, Item<'a>)>,
    engine: &Engine,
    text: &'a str,
    bidi: &BidiInfo<'a>,
    range: Range,
    styles: StyleChain<'a>,
) {
    let script = TextElem::script_in(styles);
    let lang = TextElem::lang_in(styles);
    let region = TextElem::region_in(styles);
    let mut process = |range: Range, level: BidiLevel| {
        let dir = if level.is_ltr() { Dir::LTR } else { Dir::RTL };
        let shaped =
            shape(engine, range.start, &text[range.clone()], styles, dir, lang, region);
        items.push((range, Item::Text(shaped)));
    };

    let mut prev_level = BidiLevel::ltr();
    let mut prev_script = Script::Unknown;
    let mut cursor = range.start;

    // Group by embedding level and script.  If the text's script is explicitly
    // set (rather than inferred from the glyphs), we keep the script at an
    // unchanging `Script::Unknown` so that only level changes cause breaks.
    for i in range.clone() {
        if !text.is_char_boundary(i) {
            continue;
        }

        let level = bidi.levels[i];
        let curr_script = match script {
            Smart::Auto => {
                text[i..].chars().next().map_or(Script::Unknown, |c| c.script())
            }
            Smart::Custom(_) => Script::Unknown,
        };

        if level != prev_level || !is_compatible(curr_script, prev_script) {
            if cursor < i {
                process(cursor..i, prev_level);
            }
            cursor = i;
            prev_level = level;
            prev_script = curr_script;
        } else if is_generic_script(prev_script) {
            prev_script = curr_script;
        }
    }

    process(cursor..range.end, prev_level);
}

/// Whether this is not a specific script.
fn is_generic_script(script: Script) -> bool {
    matches!(script, Script::Unknown | Script::Common | Script::Inherited)
}

/// Whether these script can be part of the same shape run.
fn is_compatible(a: Script, b: Script) -> bool {
    is_generic_script(a) || is_generic_script(b) || a == b
}

/// Shape text into [`ShapedText`].
#[allow(clippy::too_many_arguments)]
fn shape<'a>(
    engine: &Engine,
    base: usize,
    text: &'a str,
    styles: StyleChain<'a>,
    dir: Dir,
    lang: Lang,
    region: Option<Region>,
) -> ShapedText<'a> {
    let size = TextElem::size_in(styles);
    let mut ctx = ShapingContext {
        engine,
        size,
        glyphs: vec![],
        used: vec![],
        styles,
        variant: variant(styles),
        features: features(styles),
        fallback: TextElem::fallback_in(styles),
        dir,
    };

    if !text.is_empty() {
        shape_segment(&mut ctx, base, text, families(styles));
    }

    track_and_space(&mut ctx);
    calculate_adjustability(&mut ctx, lang, region);

    #[cfg(debug_assertions)]
    assert_all_glyphs_in_range(&ctx.glyphs, text, base..(base + text.len()));
    #[cfg(debug_assertions)]
    assert_glyph_ranges_in_order(&ctx.glyphs, dir);

    ShapedText {
        base,
        text,
        dir,
        lang,
        region,
        styles,
        variant: ctx.variant,
        size,
        width: ctx.glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(size),
        glyphs: Cow::Owned(ctx.glyphs),
    }
}

/// Holds shaping results and metadata common to all shaped segments.
struct ShapingContext<'a, 'v> {
    engine: &'a Engine<'v>,
    glyphs: Vec<ShapedGlyph>,
    used: Vec<Font>,
    styles: StyleChain<'a>,
    size: Abs,
    variant: FontVariant,
    features: Vec<rustybuzz::Feature>,
    fallback: bool,
    dir: Dir,
}

/// Shape text with font fallback using the `families` iterator.
fn shape_segment<'a>(
    ctx: &mut ShapingContext,
    base: usize,
    text: &str,
    mut families: impl Iterator<Item = &'a FontFamily> + Clone,
) {
    // Don't try shaping newlines, tabs, or default ignorables.
    if text
        .chars()
        .all(|c| c == '\n' || c == '\t' || is_default_ignorable(c))
    {
        return;
    }

    // Find the next available family.
    let world = ctx.engine.world;
    let book = world.book();
    let mut selection = None;
    let mut coverage = None;
    for family in families.by_ref() {
        selection = book
            .select(family.as_str(), ctx.variant)
            .and_then(|id| world.font(id))
            .filter(|font| !ctx.used.contains(font));
        if selection.is_some() {
            coverage = family.coverage();
            break;
        }
    }

    // Do font fallback if the families are exhausted and fallback is enabled.
    if selection.is_none() && ctx.fallback {
        let first = ctx.used.first().map(Font::info);
        selection = book
            .select_fallback(first, ctx.variant, text)
            .and_then(|id| world.font(id))
            .filter(|font| !ctx.used.contains(font));
    }

    // Extract the font id or shape notdef glyphs if we couldn't find any font.
    let Some(font) = selection else {
        if let Some(font) = ctx.used.first().cloned() {
            shape_tofus(ctx, base, text, font);
        }
        return;
    };

    ctx.used.push(font.clone());

    // Fill the buffer with our text.
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_language(language(ctx.styles));
    if let Some(script) = TextElem::script_in(ctx.styles).custom().and_then(|script| {
        rustybuzz::Script::from_iso15924_tag(Tag::from_bytes(script.as_bytes()))
    }) {
        buffer.set_script(script)
    }
    buffer.set_direction(match ctx.dir {
        Dir::LTR => rustybuzz::Direction::LeftToRight,
        Dir::RTL => rustybuzz::Direction::RightToLeft,
        _ => unimplemented!("vertical text layout"),
    });
    buffer.guess_segment_properties();

    // By default, Harfbuzz will create zero-width space glyphs for default
    // ignorables. This is probably useful for GUI apps that want noticeable
    // effects on the cursor for those, but for us it's not useful and hurts
    // text extraction.
    buffer.set_flags(BufferFlags::REMOVE_DEFAULT_IGNORABLES);

    // Prepare the shape plan. This plan depends on direction, script, language,
    // and features, but is independent from the text and can thus be memoized.
    let plan = create_shape_plan(
        &font,
        buffer.direction(),
        buffer.script(),
        buffer.language().as_ref(),
        &ctx.features,
    );

    // Shape!
    let buffer = rustybuzz::shape_with_plan(font.rusty(), &plan, buffer);
    let infos = buffer.glyph_infos();
    let pos = buffer.glyph_positions();
    let ltr = ctx.dir.is_positive();

    let char_in_coverage = |char_start| {
        let char_end = text[char_start..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| offset + char_start)
            .unwrap_or(text.len());
        coverage.map_or(true, |cov| cov.is_match(&text[char_start..char_end]))
    };

    // Collect the shaped glyphs, doing fallback and shaping parts again with
    // the next font if necessary.
    let mut i = 0;
    while i < infos.len() {
        let info = &infos[i];
        let cluster = info.cluster as usize;

        // Add the glyph to the shaped output.
        if info.glyph_id != 0 && char_in_coverage(cluster) {
            // Determine the text range of the glyph.
            let start = base + cluster;
            let end = base
                + if ltr { i.checked_add(1) } else { i.checked_sub(1) }
                    .and_then(|last| infos.get(last))
                    .map_or(text.len(), |info| info.cluster as usize);

            let c = text[cluster..].chars().next().unwrap();

            let script = c.script();
            let x_advance = font.to_em(pos[i].x_advance);
            ctx.glyphs.push(ShapedGlyph {
                font: font.clone(),
                glyph_id: info.glyph_id as u16,
                // TODO: Don't ignore y_advance.
                x_advance,
                x_offset: font.to_em(pos[i].x_offset),
                y_offset: font.to_em(pos[i].y_offset),
                adjustability: Adjustability::default(),
                range: start..end,
                safe_to_break: !info.unsafe_to_break(),
                c,
                is_justifiable: is_justifiable(
                    c,
                    script,
                    x_advance,
                    Adjustability::default().stretchability,
                ),
                script,
            });
        } else {
            // First, search for the end of the tofu sequence.
            let k = i;
            while infos.get(i + 1).is_some_and(|info| {
                info.glyph_id == 0 || !char_in_coverage(info.cluster as _)
            }) {
                i += 1;
            }

            // Then, determine the start and end text index for the tofu
            // sequence.
            //
            // Examples:
            // Everything is shown in visual order. Tofus are written as "_".
            // We want to find out that the tofus span the text `2..6`.
            // Note that the clusters are longer than 1 char.
            //
            // Left-to-right:
            // Text:     h a l i h a l l o
            // Glyphs:   A   _   _   C   E
            // Clusters: 0   2   4   6   8
            //              k=1 i=2
            //
            // Right-to-left:
            // Text:     O L L A H I L A H
            // Glyphs:   E   C   _   _   A
            // Clusters: 8   6   4   2   0
            //                  k=2 i=3
            let start = infos[if ltr { k } else { i }].cluster as usize;
            let end = if ltr { i.checked_add(1) } else { k.checked_sub(1) }
                .and_then(|last| infos.get(last))
                .map_or(text.len(), |info| info.cluster as usize);

            // Trim half-baked cluster.
            let remove = base + start..base + end;
            while ctx.glyphs.last().is_some_and(|g| remove.contains(&g.range.start)) {
                ctx.glyphs.pop();
            }

            // Recursively shape the tofu sequence with the next family.
            shape_segment(ctx, base + start, &text[start..end], families.clone());
        }

        i += 1;
    }

    ctx.used.pop();
}

/// Create a shape plan.
#[comemo::memoize]
fn create_shape_plan(
    font: &Font,
    direction: rustybuzz::Direction,
    script: rustybuzz::Script,
    language: Option<&rustybuzz::Language>,
    features: &[rustybuzz::Feature],
) -> Arc<ShapePlan> {
    Arc::new(rustybuzz::ShapePlan::new(
        font.rusty(),
        direction,
        Some(script),
        language,
        features,
    ))
}

/// Shape the text with tofus from the given font.
fn shape_tofus(ctx: &mut ShapingContext, base: usize, text: &str, font: Font) {
    let x_advance = font.advance(0).unwrap_or_default();
    let add_glyph = |(cluster, c): (usize, char)| {
        let start = base + cluster;
        let end = start + c.len_utf8();
        let script = c.script();
        ctx.glyphs.push(ShapedGlyph {
            font: font.clone(),
            glyph_id: 0,
            x_advance,
            x_offset: Em::zero(),
            y_offset: Em::zero(),
            adjustability: Adjustability::default(),
            range: start..end,
            safe_to_break: true,
            c,
            is_justifiable: is_justifiable(
                c,
                script,
                x_advance,
                Adjustability::default().stretchability,
            ),
            script,
        });
    };
    if ctx.dir.is_positive() {
        text.char_indices().for_each(add_glyph);
    } else {
        text.char_indices().rev().for_each(add_glyph);
    }
}

/// Apply tracking and spacing to the shaped glyphs.
fn track_and_space(ctx: &mut ShapingContext) {
    let tracking = Em::from_length(TextElem::tracking_in(ctx.styles), ctx.size);
    let spacing =
        TextElem::spacing_in(ctx.styles).map(|abs| Em::from_length(abs, ctx.size));

    let mut glyphs = ctx.glyphs.iter_mut().peekable();
    while let Some(glyph) = glyphs.next() {
        // Make non-breaking space same width as normal space.
        if glyph.c == '\u{00A0}' {
            glyph.x_advance -= nbsp_delta(&glyph.font).unwrap_or_default();
        }

        if glyph.is_space() {
            glyph.x_advance = spacing.relative_to(glyph.x_advance);
        }

        if glyphs
            .peek()
            .is_some_and(|next| glyph.range.start != next.range.start)
        {
            glyph.x_advance += tracking;
        }
    }
}

/// Calculate stretchability and shrinkability of each glyph,
/// and CJK punctuation adjustments according to Chinese Layout Requirements.
fn calculate_adjustability(ctx: &mut ShapingContext, lang: Lang, region: Option<Region>) {
    let style = cjk_punct_style(lang, region);

    for glyph in &mut ctx.glyphs {
        glyph.adjustability = glyph.base_adjustability(style);
    }

    let mut glyphs = ctx.glyphs.iter_mut().peekable();
    while let Some(glyph) = glyphs.next() {
        // CNS style needs not further adjustment.
        if glyph.is_cjk_punctuation() && matches!(style, CjkPunctStyle::Cns) {
            continue;
        }

        // Now we apply consecutive punctuation adjustment, specified in Chinese Layout.
        // Requirements, section 3.1.6.1 Punctuation Adjustment Space, and Japanese Layout
        // Requirements, section 3.1 Line Composition Rules for Punctuation Marks
        let Some(next) = glyphs.peek_mut() else { continue };
        let width = glyph.x_advance;
        let delta = width / 2.0;
        if glyph.is_cjk_punctuation()
            && next.is_cjk_punctuation()
            && (glyph.shrinkability().1 + next.shrinkability().0) >= delta
        {
            let left_delta = glyph.shrinkability().1.min(delta);
            glyph.shrink_right(left_delta);
            next.shrink_left(delta - left_delta);
        }
    }
}

/// Difference between non-breaking and normal space.
fn nbsp_delta(font: &Font) -> Option<Em> {
    let space = font.ttf().glyph_index(' ')?.0;
    let nbsp = font.ttf().glyph_index('\u{00A0}')?.0;
    Some(font.advance(nbsp)? - font.advance(space)?)
}

/// Process the language and region of a style chain into a
/// rustybuzz-compatible BCP 47 language.
fn language(styles: StyleChain) -> rustybuzz::Language {
    let mut bcp: EcoString = TextElem::lang_in(styles).as_str().into();
    if let Some(region) = TextElem::region_in(styles) {
        bcp.push('-');
        bcp.push_str(region.as_str());
    }
    rustybuzz::Language::from_str(&bcp).unwrap()
}

/// Returns true if all glyphs in `glyphs` have ranges within the range `range`.
#[cfg(debug_assertions)]
fn assert_all_glyphs_in_range(glyphs: &[ShapedGlyph], text: &str, range: Range) {
    if glyphs
        .iter()
        .any(|g| g.range.start < range.start || g.range.end > range.end)
    {
        panic!("one or more glyphs in {text:?} fell out of range");
    }
}

/// Asserts that the ranges of `glyphs` is in the proper order according to
/// `dir`.
///
/// This asserts instead of returning a bool in order to provide a more
/// informative message when the invariant is violated.
#[cfg(debug_assertions)]
fn assert_glyph_ranges_in_order(glyphs: &[ShapedGlyph], dir: Dir) {
    if glyphs.is_empty() {
        return;
    }

    // Iterator::is_sorted and friends are unstable as of Rust 1.70.0
    for i in 0..(glyphs.len() - 1) {
        let a = &glyphs[i];
        let b = &glyphs[i + 1];
        let ord = a.range.start.cmp(&b.range.start);
        let ord = if dir.is_positive() { ord } else { ord.reverse() };
        if ord == std::cmp::Ordering::Greater {
            panic!(
                "glyph ranges should be monotonically {}, \
                 but found glyphs out of order:\n\n\
                 first: {a:#?}\nsecond: {b:#?}",
                if dir.is_positive() { "increasing" } else { "decreasing" },
            );
        }
    }
}

// The CJK punctuation that can appear at the beginning or end of a line.
pub const BEGIN_PUNCT_PAT: &[char] =
    &['“', '‘', '《', '〈', '（', '『', '「', '【', '〖', '〔', '［', '｛'];
pub const END_PUNCT_PAT: &[char] = &[
    '”', '’', '，', '．', '。', '、', '：', '；', '》', '〉', '）', '』', '」', '】',
    '〗', '〕', '］', '｝', '？', '！',
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CjkPunctStyle {
    /// Standard GB/T 15834-2011, used mostly in mainland China.
    Gb,
    /// Standard by Taiwan Ministry of Education, used in Taiwan and Hong Kong.
    Cns,
    /// Standard JIS X 4051, used in Japan.
    Jis,
}

pub fn cjk_punct_style(lang: Lang, region: Option<Region>) -> CjkPunctStyle {
    match (lang, region.as_ref().map(Region::as_str)) {
        (Lang::CHINESE, Some("TW" | "HK")) => CjkPunctStyle::Cns,
        (Lang::JAPANESE, _) => CjkPunctStyle::Jis,
        // zh-CN, zh-SG, zh-MY use GB-style punctuation,
        _ => CjkPunctStyle::Gb,
    }
}

/// Whether the glyph is a space.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\u{00A0}' | '　')
}

/// Whether the glyph is part of Chinese or Japanese script (i.e. CJ, not CJK).
pub fn is_of_cj_script(c: char) -> bool {
    is_cj_script(c, c.script())
}

/// Whether the glyph is part of Chinese or Japanese script (i.e. CJ, not CJK).
/// The function is dedicated to typesetting Chinese or Japanese, which do not
/// have spaces between words, so K is not checked here.
fn is_cj_script(c: char, script: Script) -> bool {
    use Script::*;
    // U+30FC: Katakana-Hiragana Prolonged Sound Mark
    matches!(script, Hiragana | Katakana | Han) || c == '\u{30FC}'
}

/// See <https://www.w3.org/TR/clreq/#punctuation_width_adjustment>
fn is_cjk_left_aligned_punctuation(
    c: char,
    x_advance: Em,
    stretchability: (Em, Em),
    style: CjkPunctStyle,
) -> bool {
    use CjkPunctStyle::*;

    // CJK quotation marks shares codepoints with latin quotation marks.
    // But only the CJK ones have full width.
    if matches!(c, '”' | '’') && x_advance + stretchability.1 == Em::one() {
        return true;
    }

    if matches!(style, Gb | Jis) && matches!(c, '，' | '。' | '．' | '、' | '：' | '；')
    {
        return true;
    }

    if matches!(style, Gb) && matches!(c, '？' | '！') {
        // In GB style, exclamations and question marks are also left aligned
        // and can be adjusted. Note that they are not adjustable in other
        // styles.
        return true;
    }

    // See appendix A.3 https://www.w3.org/TR/clreq/#tables_of_chinese_punctuation_marks
    matches!(c, '》' | '）' | '』' | '」' | '】' | '〗' | '〕' | '〉' | '］' | '｝')
}

/// See <https://www.w3.org/TR/clreq/#punctuation_width_adjustment>
fn is_cjk_right_aligned_punctuation(
    c: char,
    x_advance: Em,
    stretchability: (Em, Em),
) -> bool {
    // CJK quotation marks shares codepoints with latin quotation marks.
    // But only the CJK ones have full width.
    if matches!(c, '“' | '‘') && x_advance + stretchability.0 == Em::one() {
        return true;
    }
    // See appendix A.3 https://www.w3.org/TR/clreq/#tables_of_chinese_punctuation_marks
    matches!(c, '《' | '（' | '『' | '「' | '【' | '〖' | '〔' | '〈' | '［' | '｛')
}

/// See <https://www.w3.org/TR/clreq/#punctuation_width_adjustment>
fn is_cjk_center_aligned_punctuation(c: char, style: CjkPunctStyle) -> bool {
    if matches!(style, CjkPunctStyle::Cns)
        && matches!(c, '，' | '。' | '．' | '、' | '：' | '；')
    {
        return true;
    }

    // U+30FB: Katakana Middle Dot
    // U+00B7: Middle Dot
    matches!(c, '\u{30FB}' | '\u{00B7}')
}

/// Whether the glyph is justifiable.
///
/// Quotations in latin script and CJK are unfortunately the same codepoint
/// (U+2018, U+2019, U+201C, U+201D), but quotations in Chinese must be
/// fullwidth. This heuristics can therefore fail for monospace latin fonts.
/// However, since monospace fonts are usually not justified this edge case
/// should be rare enough.
fn is_justifiable(
    c: char,
    script: Script,
    x_advance: Em,
    stretchability: (Em, Em),
) -> bool {
    // punctuation style is not relevant here.
    let style = CjkPunctStyle::Gb;
    is_space(c)
        || is_cj_script(c, script)
        || is_cjk_left_aligned_punctuation(c, x_advance, stretchability, style)
        || is_cjk_right_aligned_punctuation(c, x_advance, stretchability)
        || is_cjk_center_aligned_punctuation(c, style)
}

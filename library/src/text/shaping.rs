use std::ops::Range;
use std::str::FromStr;

use rustybuzz::{Feature, Tag, UnicodeBuffer};
use typst::font::{Font, FontVariant};
use typst::util::SliceExt;

use super::*;
use crate::prelude::*;

/// The result of shaping text.
///
/// This type contains owned or borrowed shaped text runs, which can be
/// measured, used to reshape substrings more quickly and converted into a
/// frame.
pub(super) struct ShapedText<'a> {
    /// The text that was shaped.
    pub text: &'a str,
    /// The text direction.
    pub dir: Dir,
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
pub(super) struct ShapedGlyph {
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
    /// The byte index in the source text where this glyph's cluster starts. A
    /// cluster is a sequence of one or multiple glyphs that cannot be
    /// separated and must always be treated as a union.
    pub cluster: usize,
    /// Whether splitting the shaping result before this glyph would yield the
    /// same results as shaping the parts to both sides of `text_index`
    /// separately.
    pub safe_to_break: bool,
    /// The first char in this glyph's cluster.
    pub c: char,
}

impl ShapedGlyph {
    /// Whether the glyph is a space.
    pub fn is_space(&self) -> bool {
        matches!(self.c, ' ' | '\u{00A0}' | '　')
    }

    /// Whether the glyph is justifiable.
    pub fn is_justifiable(&self) -> bool {
        self.is_space() || matches!(self.c, '，' | '。' | '、')
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
    pub fn build(&self, world: Tracked<dyn World>, justification: Abs) -> Frame {
        let (top, bottom) = self.measure(world);
        let size = Size::new(self.width, top + bottom);

        let mut offset = Abs::zero();
        let mut frame = Frame::new(size);
        frame.set_baseline(top);

        let shift = self.styles.get(TextNode::BASELINE);
        let lang = self.styles.get(TextNode::LANG);
        let decos = self.styles.get(TextNode::DECO);
        let fill = self.styles.get(TextNode::FILL);
        let link = self.styles.get(TextNode::LINK);

        for ((font, y_offset), group) in
            self.glyphs.as_ref().group_by_key(|g| (g.font.clone(), g.y_offset))
        {
            let pos = Point::new(offset, top + shift + y_offset.at(self.size));
            let glyphs = group
                .iter()
                .map(|glyph| Glyph {
                    id: glyph.glyph_id,
                    x_advance: glyph.x_advance
                        + if glyph.is_justifiable() {
                            frame.size_mut().x += justification;
                            Em::from_length(justification, self.size)
                        } else {
                            Em::zero()
                        },
                    x_offset: glyph.x_offset,
                    c: glyph.c,
                })
                .collect();

            let text = Text { font, size: self.size, lang, fill, glyphs };
            let text_layer = frame.layer();
            let width = text.width();

            // Apply line decorations.
            for deco in &decos {
                decorate(&mut frame, deco, &text, shift, pos, width);
            }

            frame.insert(text_layer, pos, Element::Text(text));
            offset += width;
        }

        // Apply link if it exists.
        if let Some(dest) = link {
            frame.link(dest.clone());
        }

        frame
    }

    /// Measure the top and bottom extent of this text.
    fn measure(&self, world: Tracked<dyn World>) -> (Abs, Abs) {
        let mut top = Abs::zero();
        let mut bottom = Abs::zero();

        let top_edge = self.styles.get(TextNode::TOP_EDGE);
        let bottom_edge = self.styles.get(TextNode::BOTTOM_EDGE);

        // Expand top and bottom by reading the font's vertical metrics.
        let mut expand = |font: &Font| {
            let metrics = font.metrics();
            top.set_max(top_edge.resolve(self.styles, metrics));
            bottom.set_max(-bottom_edge.resolve(self.styles, metrics));
        };

        if self.glyphs.is_empty() {
            // When there are no glyphs, we just use the vertical metrics of the
            // first available font.
            for family in families(self.styles) {
                if let Some(font) = world
                    .book()
                    .select(family, self.variant)
                    .and_then(|id| world.font(id))
                {
                    expand(&font);
                    break;
                }
            }
        } else {
            for g in self.glyphs.iter() {
                expand(&g.font);
            }
        }

        (top, bottom)
    }

    /// How many justifiable glyphs the text contains.
    pub fn justifiables(&self) -> usize {
        self.glyphs.iter().filter(|g| g.is_justifiable()).count()
    }

    /// The width of the spaces in the text.
    pub fn stretch(&self) -> Abs {
        self.glyphs
            .iter()
            .filter(|g| g.is_justifiable())
            .map(|g| g.x_advance)
            .sum::<Em>()
            .at(self.size)
    }

    /// Reshape a range of the shaped text, reusing information from this
    /// shaping process if possible.
    pub fn reshape(
        &'a self,
        world: Tracked<dyn World>,
        text_range: Range<usize>,
    ) -> ShapedText<'a> {
        if let Some(glyphs) = self.slice_safe_to_break(text_range.clone()) {
            Self {
                text: &self.text[text_range],
                dir: self.dir,
                styles: self.styles,
                size: self.size,
                variant: self.variant,
                width: glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(self.size),
                glyphs: Cow::Borrowed(glyphs),
            }
        } else {
            shape(world, &self.text[text_range], self.styles, self.dir)
        }
    }

    /// Push a hyphen to end of the text.
    pub fn push_hyphen(&mut self, world: Tracked<dyn World>) {
        families(self.styles).find_map(|family| {
            let font = world
                .book()
                .select(family, self.variant)
                .and_then(|id| world.font(id))?;
            let ttf = font.ttf();
            let glyph_id = ttf.glyph_index('-')?;
            let x_advance = font.to_em(ttf.glyph_hor_advance(glyph_id)?);
            let cluster = self.glyphs.last().map(|g| g.cluster).unwrap_or_default();
            self.width += x_advance.at(self.size);
            self.glyphs.to_mut().push(ShapedGlyph {
                font,
                glyph_id: glyph_id.0,
                x_advance,
                x_offset: Em::zero(),
                y_offset: Em::zero(),
                cluster,
                safe_to_break: true,
                c: '-',
            });
            Some(())
        });
    }

    /// Find the subslice of glyphs that represent the given text range if both
    /// sides are safe to break.
    fn slice_safe_to_break(&self, text_range: Range<usize>) -> Option<&[ShapedGlyph]> {
        let Range { mut start, mut end } = text_range;
        if !self.dir.is_positive() {
            std::mem::swap(&mut start, &mut end);
        }

        let left = self.find_safe_to_break(start, Side::Left)?;
        let right = self.find_safe_to_break(end, Side::Right)?;
        Some(&self.glyphs[left..right])
    }

    /// Find the glyph offset matching the text index that is most towards the
    /// given side and safe-to-break.
    fn find_safe_to_break(&self, text_index: usize, towards: Side) -> Option<usize> {
        let ltr = self.dir.is_positive();

        // Handle edge cases.
        let len = self.glyphs.len();
        if text_index == 0 {
            return Some(if ltr { 0 } else { len });
        } else if text_index == self.text.len() {
            return Some(if ltr { len } else { 0 });
        }

        // Find any glyph with the text index.
        let mut idx = self
            .glyphs
            .binary_search_by(|g| {
                let ordering = g.cluster.cmp(&text_index);
                if ltr {
                    ordering
                } else {
                    ordering.reverse()
                }
            })
            .ok()?;

        let next = match towards {
            Side::Left => usize::checked_sub,
            Side::Right => usize::checked_add,
        };

        // Search for the outermost glyph with the text index.
        while let Some(next) = next(idx, 1) {
            if self.glyphs.get(next).map_or(true, |g| g.cluster != text_index) {
                break;
            }
            idx = next;
        }

        // RTL needs offset one because the left side of the range should be
        // exclusive and the right side inclusive, contrary to the normal
        // behaviour of ranges.
        self.glyphs[idx].safe_to_break.then(|| idx + (!ltr) as usize)
    }
}

impl Debug for ShapedText<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.text.fmt(f)
    }
}

/// Holds shaping results and metadata common to all shaped segments.
struct ShapingContext<'a> {
    world: Tracked<'a, dyn World>,
    glyphs: Vec<ShapedGlyph>,
    used: Vec<Font>,
    styles: StyleChain<'a>,
    size: Abs,
    variant: FontVariant,
    tags: Vec<rustybuzz::Feature>,
    fallback: bool,
    dir: Dir,
}

/// Shape text into [`ShapedText`].
pub(super) fn shape<'a>(
    world: Tracked<dyn World>,
    text: &'a str,
    styles: StyleChain<'a>,
    dir: Dir,
) -> ShapedText<'a> {
    let size = styles.get(TextNode::SIZE);

    let mut ctx = ShapingContext {
        world,
        size,
        glyphs: vec![],
        used: vec![],
        styles,
        variant: variant(styles),
        tags: tags(styles),
        fallback: styles.get(TextNode::FALLBACK),
        dir,
    };

    if !text.is_empty() {
        shape_segment(&mut ctx, 0, text, families(styles));
    }

    track_and_space(&mut ctx);

    ShapedText {
        text,
        dir,
        styles,
        variant: ctx.variant,
        size,
        width: ctx.glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(size),
        glyphs: Cow::Owned(ctx.glyphs),
    }
}

/// Shape text with font fallback using the `families` iterator.
fn shape_segment<'a>(
    ctx: &mut ShapingContext,
    base: usize,
    text: &str,
    mut families: impl Iterator<Item = &'a str> + Clone,
) {
    // Fonts dont have newlines and tabs.
    if text.chars().all(|c| c == '\n' || c == '\t') {
        return;
    }

    // Find the next available family.
    let book = ctx.world.book();
    let mut selection = families.find_map(|family| {
        book.select(family, ctx.variant)
            .and_then(|id| ctx.world.font(id))
            .filter(|font| !ctx.used.contains(font))
    });

    // Do font fallback if the families are exhausted and fallback is enabled.
    if selection.is_none() && ctx.fallback {
        let first = ctx.used.first().map(Font::info);
        selection = book
            .select_fallback(first, ctx.variant, text)
            .and_then(|id| ctx.world.font(id))
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
    buffer.set_direction(match ctx.dir {
        Dir::LTR => rustybuzz::Direction::LeftToRight,
        Dir::RTL => rustybuzz::Direction::RightToLeft,
        _ => unimplemented!("vertical text layout"),
    });

    // Shape!
    let buffer = rustybuzz::shape(font.rusty(), &ctx.tags, buffer);
    let infos = buffer.glyph_infos();
    let pos = buffer.glyph_positions();

    // Collect the shaped glyphs, doing fallback and shaping parts again with
    // the next font if necessary.
    let mut i = 0;
    while i < infos.len() {
        let info = &infos[i];
        let cluster = info.cluster as usize;

        if info.glyph_id != 0 {
            // Add the glyph to the shaped output.
            // TODO: Don't ignore y_advance.
            ctx.glyphs.push(ShapedGlyph {
                font: font.clone(),
                glyph_id: info.glyph_id as u16,
                x_advance: font.to_em(pos[i].x_advance),
                x_offset: font.to_em(pos[i].x_offset),
                y_offset: font.to_em(pos[i].y_offset),
                cluster: base + cluster,
                safe_to_break: !info.unsafe_to_break(),
                c: text[cluster..].chars().next().unwrap(),
            });
        } else {
            // Determine the source text range for the tofu sequence.
            let range = {
                // First, search for the end of the tofu sequence.
                let k = i;
                while infos.get(i + 1).map_or(false, |info| info.glyph_id == 0) {
                    i += 1;
                }

                // Then, determine the start and end text index.
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
                let ltr = ctx.dir.is_positive();
                let first = if ltr { k } else { i };
                let start = infos[first].cluster as usize;
                let last = if ltr { i.checked_add(1) } else { k.checked_sub(1) };
                let end = last
                    .and_then(|last| infos.get(last))
                    .map_or(text.len(), |info| info.cluster as usize);

                start..end
            };

            // Trim half-baked cluster.
            let remove = base + range.start..base + range.end;
            while ctx.glyphs.last().map_or(false, |g| remove.contains(&g.cluster)) {
                ctx.glyphs.pop();
            }

            // Recursively shape the tofu sequence with the next family.
            shape_segment(ctx, base + range.start, &text[range], families.clone());
        }

        i += 1;
    }

    ctx.used.pop();
}

/// Shape the text with tofus from the given font.
fn shape_tofus(ctx: &mut ShapingContext, base: usize, text: &str, font: Font) {
    let x_advance = font.advance(0).unwrap_or_default();
    for (cluster, c) in text.char_indices() {
        ctx.glyphs.push(ShapedGlyph {
            font: font.clone(),
            glyph_id: 0,
            x_advance,
            x_offset: Em::zero(),
            y_offset: Em::zero(),
            cluster: base + cluster,
            safe_to_break: true,
            c,
        });
    }
}

/// Apply tracking and spacing to the shaped glyphs.
fn track_and_space(ctx: &mut ShapingContext) {
    let tracking = Em::from_length(ctx.styles.get(TextNode::TRACKING), ctx.size);
    let spacing = ctx
        .styles
        .get(TextNode::SPACING)
        .map(|abs| Em::from_length(abs, ctx.size));

    let mut glyphs = ctx.glyphs.iter_mut().peekable();
    while let Some(glyph) = glyphs.next() {
        // Make non-breaking space same width as normal space.
        if glyph.c == '\u{00A0}' {
            glyph.x_advance -= nbsp_delta(&glyph.font).unwrap_or_default();
        }

        if glyph.is_space() {
            glyph.x_advance = spacing.relative_to(glyph.x_advance);
        }

        if glyphs.peek().map_or(false, |next| glyph.cluster != next.cluster) {
            glyph.x_advance += tracking;
        }
    }
}

/// Difference between non-breaking and normal space.
fn nbsp_delta(font: &Font) -> Option<Em> {
    let space = font.ttf().glyph_index(' ')?.0;
    let nbsp = font.ttf().glyph_index('\u{00A0}')?.0;
    Some(font.advance(nbsp)? - font.advance(space)?)
}

/// Resolve the font variant.
pub fn variant(styles: StyleChain) -> FontVariant {
    let mut variant = FontVariant::new(
        styles.get(TextNode::STYLE),
        styles.get(TextNode::WEIGHT),
        styles.get(TextNode::STRETCH),
    );

    if styles.get(TextNode::BOLD) {
        variant.weight = variant.weight.thicken(300);
    }

    if styles.get(TextNode::ITALIC) {
        variant.style = match variant.style {
            FontStyle::Normal => FontStyle::Italic,
            FontStyle::Italic => FontStyle::Normal,
            FontStyle::Oblique => FontStyle::Normal,
        }
    }

    variant
}

/// Resolve a prioritized iterator over the font families.
fn families(styles: StyleChain) -> impl Iterator<Item = &str> + Clone {
    const FALLBACKS: &[&str] = &[
        "ibm plex sans",
        "twitter color emoji",
        "noto color emoji",
        "apple color emoji",
        "segoe ui emoji",
    ];

    let tail = if styles.get(TextNode::FALLBACK) { FALLBACKS } else { &[] };
    styles
        .get(TextNode::FAMILY)
        .0
        .iter()
        .map(|family| family.as_str())
        .chain(tail.iter().copied())
}

/// Collect the tags of the OpenType features to apply.
fn tags(styles: StyleChain) -> Vec<Feature> {
    let mut tags = vec![];
    let mut feat = |tag, value| {
        tags.push(Feature::new(Tag::from_bytes(tag), value, ..));
    };

    // Features that are on by default in Harfbuzz are only added if disabled.
    if !styles.get(TextNode::KERNING) {
        feat(b"kern", 0);
    }

    // Features that are off by default in Harfbuzz are only added if enabled.
    if styles.get(TextNode::SMALLCAPS) {
        feat(b"smcp", 1);
    }

    if styles.get(TextNode::ALTERNATES) {
        feat(b"salt", 1);
    }

    let storage;
    if let Some(set) = styles.get(TextNode::STYLISTIC_SET) {
        storage = [b's', b's', b'0' + set.get() / 10, b'0' + set.get() % 10];
        feat(&storage, 1);
    }

    if !styles.get(TextNode::LIGATURES) {
        feat(b"liga", 0);
        feat(b"clig", 0);
    }

    if styles.get(TextNode::DISCRETIONARY_LIGATURES) {
        feat(b"dlig", 1);
    }

    if styles.get(TextNode::HISTORICAL_LIGATURES) {
        feat(b"hilg", 1);
    }

    match styles.get(TextNode::NUMBER_TYPE) {
        Smart::Auto => {}
        Smart::Custom(NumberType::Lining) => feat(b"lnum", 1),
        Smart::Custom(NumberType::OldStyle) => feat(b"onum", 1),
    }

    match styles.get(TextNode::NUMBER_WIDTH) {
        Smart::Auto => {}
        Smart::Custom(NumberWidth::Proportional) => feat(b"pnum", 1),
        Smart::Custom(NumberWidth::Tabular) => feat(b"tnum", 1),
    }

    if styles.get(TextNode::SLASHED_ZERO) {
        feat(b"zero", 1);
    }

    if styles.get(TextNode::FRACTIONS) {
        feat(b"frac", 1);
    }

    for (tag, value) in styles.get(TextNode::FEATURES).0 {
        tags.push(Feature::new(tag, value, ..))
    }

    tags
}

/// Process the language and and region of a style chain into a
/// rustybuzz-compatible BCP 47 language.
fn language(styles: StyleChain) -> rustybuzz::Language {
    let mut bcp: EcoString = styles.get(TextNode::LANG).as_str().into();
    if let Some(region) = styles.get(TextNode::REGION) {
        bcp.push('-');
        bcp.push_str(region.as_str());
    }
    rustybuzz::Language::from_str(&bcp).unwrap()
}

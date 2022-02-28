use std::ops::Range;

use rustybuzz::{Feature, UnicodeBuffer};

use super::*;
use crate::font::{FaceId, FontStore, FontVariant};
use crate::library::prelude::*;
use crate::util::SliceExt;

/// The result of shaping text.
///
/// This type contains owned or borrowed shaped text runs, which can be
/// measured, used to reshape substrings more quickly and converted into a
/// frame.
#[derive(Debug, Clone)]
pub struct ShapedText<'a> {
    /// The text that was shaped.
    pub text: Cow<'a, str>,
    /// The text direction.
    pub dir: Dir,
    /// The text's style properties.
    pub styles: StyleChain<'a>,
    /// The size of the text's bounding box.
    pub size: Size,
    /// The baseline from the top of the frame.
    pub baseline: Length,
    /// The shaped glyphs.
    pub glyphs: Cow<'a, [ShapedGlyph]>,
}

/// A single glyph resulting from shaping.
#[derive(Debug, Copy, Clone)]
pub struct ShapedGlyph {
    /// The font face the glyph is contained in.
    pub face_id: FaceId,
    /// The glyph's index in the face.
    pub glyph_id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The start index of the glyph in the source text.
    pub text_index: usize,
    /// Whether splitting the shaping result before this glyph would yield the
    /// same results as shaping the parts to both sides of `text_index`
    /// separately.
    pub safe_to_break: bool,
    /// Whether this glyph represents a space.
    pub is_space: bool,
}

/// A side you can go toward.
enum Side {
    /// Go toward the west.
    Left,
    /// Go toward the east.
    Right,
}

impl<'a> ShapedText<'a> {
    /// Build the shaped text's frame.
    ///
    /// The `justification` defines how much extra advance width each
    /// [space glyph](ShapedGlyph::is_space) will get.
    pub fn build(&self, fonts: &FontStore, justification: Length) -> Frame {
        let mut offset = Length::zero();
        let mut frame = Frame::new(self.size);
        frame.baseline = Some(self.baseline);

        for (face_id, group) in self.glyphs.as_ref().group_by_key(|g| g.face_id) {
            let pos = Point::new(offset, self.baseline);

            let size = self.styles.get(TextNode::SIZE).abs;
            let fill = self.styles.get(TextNode::FILL);
            let glyphs = group
                .iter()
                .map(|glyph| Glyph {
                    id: glyph.glyph_id,
                    x_advance: glyph.x_advance
                        + if glyph.is_space {
                            frame.size.x += justification;
                            Em::from_length(justification, size)
                        } else {
                            Em::zero()
                        },
                    x_offset: glyph.x_offset,
                })
                .collect();

            let text = Text { face_id, size, fill, glyphs };
            let text_layer = frame.layer();
            let width = text.width();

            // Apply line decorations.
            for deco in self.styles.get_cloned(TextNode::LINES) {
                decorate(&mut frame, &deco, fonts, &text, pos, width);
            }

            frame.insert(text_layer, pos, Element::Text(text));
            offset += width;
        }

        // Apply link if it exists.
        if let Some(url) = self.styles.get_ref(TextNode::LINK) {
            frame.link(url);
        }

        frame
    }

    /// How many spaces the text contains.
    pub fn spaces(&self) -> usize {
        self.glyphs.iter().filter(|g| g.is_space).count()
    }

    /// Reshape a range of the shaped text, reusing information from this
    /// shaping process if possible.
    pub fn reshape(
        &'a self,
        fonts: &mut FontStore,
        text_range: Range<usize>,
    ) -> ShapedText<'a> {
        if let Some(glyphs) = self.slice_safe_to_break(text_range.clone()) {
            let (size, baseline) = measure(fonts, glyphs, self.styles);
            Self {
                text: Cow::Borrowed(&self.text[text_range]),
                dir: self.dir,
                styles: self.styles.clone(),
                size,
                baseline,
                glyphs: Cow::Borrowed(glyphs),
            }
        } else {
            shape(fonts, &self.text[text_range], self.styles.clone(), self.dir)
        }
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
        Some(&self.glyphs[left .. right])
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
                let ordering = g.text_index.cmp(&text_index);
                if ltr { ordering } else { ordering.reverse() }
            })
            .ok()?;

        let next = match towards {
            Side::Left => usize::checked_sub,
            Side::Right => usize::checked_add,
        };

        // Search for the outermost glyph with the text index.
        while let Some(next) = next(idx, 1) {
            if self.glyphs.get(next).map_or(true, |g| g.text_index != text_index) {
                break;
            }
            idx = next;
        }

        // RTL needs offset one because the left side of the range should be
        // exclusive and the right side inclusive, contrary to the normal
        // behaviour of ranges.
        if !ltr {
            idx += 1;
        }

        self.glyphs[idx].safe_to_break.then(|| idx)
    }
}

/// Shape text into [`ShapedText`].
pub fn shape<'a>(
    fonts: &mut FontStore,
    text: &'a str,
    styles: StyleChain<'a>,
    dir: Dir,
) -> ShapedText<'a> {
    let text = match styles.get(TextNode::CASE) {
        Some(case) => Cow::Owned(case.apply(text)),
        None => Cow::Borrowed(text),
    };

    let mut glyphs = vec![];
    if !text.is_empty() {
        shape_segment(
            fonts,
            &mut glyphs,
            0,
            &text,
            variant(styles),
            families(styles),
            None,
            dir,
            &tags(styles),
        );
    }

    track(&mut glyphs, styles.get(TextNode::TRACKING));
    let (size, baseline) = measure(fonts, &glyphs, styles);

    ShapedText {
        text,
        dir,
        styles,
        size,
        baseline,
        glyphs: Cow::Owned(glyphs),
    }
}

/// Resolve the font variant with `STRONG` and `EMPH` factored in.
fn variant(styles: StyleChain) -> FontVariant {
    let mut variant = FontVariant::new(
        styles.get(TextNode::STYLE),
        styles.get(TextNode::WEIGHT),
        styles.get(TextNode::STRETCH),
    );

    if styles.get(TextNode::STRONG) {
        variant.weight = variant.weight.thicken(300);
    }

    if styles.get(TextNode::EMPH) {
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
    let head = if styles.get(TextNode::MONOSPACED) {
        styles.get_ref(TextNode::MONOSPACE).as_slice()
    } else {
        &[]
    };

    let core = styles.get_ref(TextNode::FAMILY).iter().flat_map(move |family| {
        match family {
            FontFamily::Named(name) => std::slice::from_ref(name),
            FontFamily::Serif => styles.get_ref(TextNode::SERIF),
            FontFamily::SansSerif => styles.get_ref(TextNode::SANS_SERIF),
            FontFamily::Monospace => styles.get_ref(TextNode::MONOSPACE),
        }
    });

    let tail: &[&str] = if styles.get(TextNode::FALLBACK) {
        &["ibm plex sans", "latin modern math", "twitter color emoji"]
    } else {
        &[]
    };

    head.iter()
        .chain(core)
        .map(|named| named.as_str())
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

    match styles.get(TextNode::NUMBER_POSITION) {
        NumberPosition::Normal => {}
        NumberPosition::Subscript => feat(b"subs", 1),
        NumberPosition::Superscript => feat(b"sups", 1),
    }

    if styles.get(TextNode::SLASHED_ZERO) {
        feat(b"zero", 1);
    }

    if styles.get(TextNode::FRACTIONS) {
        feat(b"frac", 1);
    }

    for &(tag, value) in styles.get_ref(TextNode::FEATURES).iter() {
        tags.push(Feature::new(tag, value, ..))
    }

    tags
}

/// Shape text with font fallback using the `families` iterator.
fn shape_segment<'a>(
    fonts: &mut FontStore,
    glyphs: &mut Vec<ShapedGlyph>,
    base: usize,
    text: &str,
    variant: FontVariant,
    mut families: impl Iterator<Item = &'a str> + Clone,
    mut first_face: Option<FaceId>,
    dir: Dir,
    tags: &[rustybuzz::Feature],
) {
    // No font has newlines.
    if text.chars().all(|c| c == '\n') {
        return;
    }

    // Select the font family.
    let (face_id, fallback) = loop {
        // Try to load the next available font family.
        match families.next() {
            Some(family) => {
                if let Some(id) = fonts.select(family, variant) {
                    break (id, true);
                }
            }
            // We're out of families, so we don't do any more fallback and just
            // shape the tofus with the first face we originally used.
            None => match first_face {
                Some(id) => break (id, false),
                None => return,
            },
        }
    };

    // Remember the id if this the first available face since we use that one to
    // shape tofus.
    first_face.get_or_insert(face_id);

    // Fill the buffer with our text.
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(match dir {
        Dir::LTR => rustybuzz::Direction::LeftToRight,
        Dir::RTL => rustybuzz::Direction::RightToLeft,
        _ => unimplemented!(),
    });

    // Shape!
    let mut face = fonts.get(face_id);
    let buffer = rustybuzz::shape(face.ttf(), tags, buffer);
    let infos = buffer.glyph_infos();
    let pos = buffer.glyph_positions();

    // Collect the shaped glyphs, doing fallback and shaping parts again with
    // the next font if necessary.
    let mut i = 0;
    while i < infos.len() {
        let info = &infos[i];
        let cluster = info.cluster as usize;

        if info.glyph_id != 0 || !fallback {
            // Add the glyph to the shaped output.
            // TODO: Don't ignore y_advance and y_offset.
            glyphs.push(ShapedGlyph {
                face_id,
                glyph_id: info.glyph_id as u16,
                x_advance: face.to_em(pos[i].x_advance),
                x_offset: face.to_em(pos[i].x_offset),
                text_index: base + cluster,
                safe_to_break: !info.unsafe_to_break(),
                is_space: text[cluster ..].chars().next() == Some(' '),
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
                let ltr = dir.is_positive();
                let first = if ltr { k } else { i };
                let start = infos[first].cluster as usize;
                let last = if ltr { i.checked_add(1) } else { k.checked_sub(1) };
                let end = last
                    .and_then(|last| infos.get(last))
                    .map_or(text.len(), |info| info.cluster as usize);

                start .. end
            };

            // Recursively shape the tofu sequence with the next family.
            shape_segment(
                fonts,
                glyphs,
                base + range.start,
                &text[range],
                variant,
                families.clone(),
                first_face,
                dir,
                tags,
            );

            face = fonts.get(face_id);
        }

        i += 1;
    }
}

/// Apply tracking to a slice of shaped glyphs.
fn track(glyphs: &mut [ShapedGlyph], tracking: Em) {
    if tracking.is_zero() {
        return;
    }

    let mut glyphs = glyphs.iter_mut().peekable();
    while let Some(glyph) = glyphs.next() {
        if glyphs
            .peek()
            .map_or(false, |next| glyph.text_index != next.text_index)
        {
            glyph.x_advance += tracking;
        }
    }
}

/// Measure the size and baseline of a run of shaped glyphs with the given
/// properties.
fn measure(
    fonts: &mut FontStore,
    glyphs: &[ShapedGlyph],
    styles: StyleChain,
) -> (Size, Length) {
    let mut width = Length::zero();
    let mut top = Length::zero();
    let mut bottom = Length::zero();

    let size = styles.get(TextNode::SIZE).abs;
    let top_edge = styles.get(TextNode::TOP_EDGE);
    let bottom_edge = styles.get(TextNode::BOTTOM_EDGE);

    // Expand top and bottom by reading the face's vertical metrics.
    let mut expand = |face: &Face| {
        top.set_max(face.vertical_metric(top_edge, size));
        bottom.set_max(-face.vertical_metric(bottom_edge, size));
    };

    if glyphs.is_empty() {
        // When there are no glyphs, we just use the vertical metrics of the
        // first available font.
        for family in families(styles) {
            if let Some(face_id) = fonts.select(family, variant(styles)) {
                expand(fonts.get(face_id));
                break;
            }
        }
    } else {
        for (face_id, group) in glyphs.group_by_key(|g| g.face_id) {
            let face = fonts.get(face_id);
            expand(face);

            for glyph in group {
                width += glyph.x_advance.resolve(size);
            }
        }
    }

    (Size::new(width, top + bottom), top)
}

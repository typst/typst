use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::ops::Range;

use fontdock::FaceId;
use rustybuzz::UnicodeBuffer;
use ttf_parser::GlyphId;

use super::{Element, Frame, Glyph, Text};
use crate::env::FontLoader;
use crate::exec::FontProps;
use crate::font::FaceBuf;
use crate::geom::{Dir, Length, Point, Size};
use crate::util::SliceExt;

/// The result of shaping text.
///
/// This type contains owned or borrowed shaped text runs, which can be
/// measured, used to reshape substrings more quickly and converted into a
/// frame.
pub struct ShapedText<'a> {
    /// The text that was shaped.
    pub text: &'a str,
    /// The text direction.
    pub dir: Dir,
    /// The properties used for font selection.
    pub props: &'a FontProps,
    /// The font size.
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
    /// The glyph's ID in the face.
    pub glyph_id: GlyphId,
    /// The advance width of the glyph.
    pub x_advance: i32,
    /// The horizontal offset of the glyph.
    pub x_offset: i32,
    /// The start index of the glyph in the source text.
    pub text_index: usize,
    /// Whether splitting the shaping result before this glyph would yield the
    /// same results as shaping the parts to both sides of `text_index`
    /// separately.
    pub safe_to_break: bool,
}

/// A visual side.
enum Side {
    Left,
    Right,
}

impl<'a> ShapedText<'a> {
    /// Build the shaped text's frame.
    pub fn build(&self, loader: &mut FontLoader) -> Frame {
        let mut frame = Frame::new(self.size, self.baseline);
        let mut x = Length::ZERO;

        for (face_id, group) in self.glyphs.as_ref().group_by_key(|g| g.face_id) {
            let pos = Point::new(x, self.baseline);
            let mut text = Text {
                face_id,
                size: self.props.size,
                color: self.props.color,
                glyphs: vec![],
            };

            let face = loader.face(face_id);
            for glyph in group {
                let x_advance = face.convert(glyph.x_advance).scale(self.props.size);
                let x_offset = face.convert(glyph.x_offset).scale(self.props.size);
                text.glyphs.push(Glyph { id: glyph.glyph_id, x_advance, x_offset });
                x += x_advance;
            }

            frame.push(pos, Element::Text(text));
        }

        frame
    }

    /// Reshape a range of the shaped text, reusing information from this
    /// shaping process if possible.
    pub fn reshape(
        &'a self,
        text_range: Range<usize>,
        loader: &mut FontLoader,
    ) -> ShapedText<'a> {
        if let Some(glyphs) = self.slice_safe_to_break(text_range.clone()) {
            let (size, baseline) = measure(glyphs, loader, self.props);
            Self {
                text: &self.text[text_range],
                dir: self.dir,
                props: self.props,
                size,
                baseline,
                glyphs: Cow::Borrowed(glyphs),
            }
        } else {
            shape(&self.text[text_range], self.dir, loader, self.props)
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

impl Debug for ShapedText<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Shaped({:?})", self.text)
    }
}

/// Shape text into [`ShapedText`].
pub fn shape<'a>(
    text: &'a str,
    dir: Dir,
    loader: &mut FontLoader,
    props: &'a FontProps,
) -> ShapedText<'a> {
    let mut glyphs = vec![];
    let families = props.families.iter();
    if !text.is_empty() {
        shape_segment(&mut glyphs, 0, text, dir, loader, props, families, None);
    }

    let (size, baseline) = measure(&glyphs, loader, props);
    ShapedText {
        text,
        dir,
        props,
        size,
        baseline,
        glyphs: Cow::Owned(glyphs),
    }
}

/// Shape text with font fallback using the `families` iterator.
fn shape_segment<'a>(
    glyphs: &mut Vec<ShapedGlyph>,
    base: usize,
    text: &str,
    dir: Dir,
    loader: &mut FontLoader,
    props: &FontProps,
    mut families: impl Iterator<Item = &'a str> + Clone,
    mut first: Option<FaceId>,
) {
    // Select the font family.
    let (face_id, fallback) = loop {
        // Try to load the next available font family.
        match families.next() {
            Some(family) => match loader.query(family, props.variant) {
                Some(id) => break (id, true),
                None => {}
            },
            // We're out of families, so we don't do any more fallback and just
            // shape the tofus with the first face we originally used.
            None => match first {
                Some(id) => break (id, false),
                None => return,
            },
        }
    };

    // Register that this is the first available font.
    if first.is_none() {
        first = Some(face_id);
    }

    // Fill the buffer with our text.
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(match dir {
        Dir::LTR => rustybuzz::Direction::LeftToRight,
        Dir::RTL => rustybuzz::Direction::RightToLeft,
        _ => unimplemented!(),
    });

    // Shape!
    let buffer = rustybuzz::shape(loader.face(face_id).ttf(), &[], buffer);
    let infos = buffer.glyph_infos();
    let pos = buffer.glyph_positions();

    // Collect the shaped glyphs, reshaping with the next font if necessary.
    let mut i = 0;
    while i < infos.len() {
        let info = &infos[i];
        let cluster = info.cluster as usize;

        if info.codepoint != 0 || !fallback {
            // Add the glyph to the shaped output.
            // TODO: Don't ignore y_advance and y_offset.
            glyphs.push(ShapedGlyph {
                face_id,
                glyph_id: GlyphId(info.codepoint as u16),
                x_advance: pos[i].x_advance,
                x_offset: pos[i].x_offset,
                text_index: base + cluster,
                safe_to_break: !info.unsafe_to_break(),
            });
        } else {
            // Do font fallback if the glyph is a tofu.
            //
            // First, search for the end of the tofu sequence.
            let k = i;
            while infos.get(i + 1).map_or(false, |info| info.codepoint == 0) {
                i += 1;
            }

            // Determine the source text range for the tofu sequence.
            let range = {
                // Examples
                //
                // Here, _ is a tofu.
                // Note that the glyph cluster length is greater than 1 char!
                //
                // Left-to-right clusters:
                // h a l i h a l l o
                // A   _   _   C   E
                // 0   2   4   6   8
                //
                // Right-to-left clusters:
                // O L L A H I L A H
                // E   C   _   _   A
                // 8   6   4   2   0

                let ltr = dir.is_positive();
                let first = if ltr { k } else { i };
                let start = infos[first].cluster as usize;

                let last = if ltr { i.checked_add(1) } else { k.checked_sub(1) };
                let end = last
                    .and_then(|last| infos.get(last))
                    .map(|info| info.cluster as usize)
                    .unwrap_or(text.len());

                start .. end
            };

            // Recursively shape the tofu sequence with the next family.
            shape_segment(
                glyphs,
                base + range.start,
                &text[range],
                dir,
                loader,
                props,
                families.clone(),
                first,
            );
        }

        i += 1;
    }
}

/// Measure the size and baseline of a run of shaped glyphs with the given
/// properties.
fn measure(
    glyphs: &[ShapedGlyph],
    loader: &mut FontLoader,
    props: &FontProps,
) -> (Size, Length) {
    let mut top = Length::ZERO;
    let mut bottom = Length::ZERO;
    let mut width = Length::ZERO;
    let mut vertical = |face: &FaceBuf| {
        top = top.max(face.vertical_metric(props.top_edge).scale(props.size));
        bottom = bottom.max(-face.vertical_metric(props.bottom_edge).scale(props.size));
    };

    if glyphs.is_empty() {
        // When there are no glyphs, we just use the vertical metrics of the
        // first available font.
        for family in props.families.iter() {
            if let Some(face_id) = loader.query(family, props.variant) {
                vertical(loader.face(face_id));
                break;
            }
        }
    } else {
        for (face_id, group) in glyphs.group_by_key(|g| g.face_id) {
            let face = loader.face(face_id);
            vertical(face);

            for glyph in group {
                width += face.convert(glyph.x_advance).scale(props.size);
            }
        }
    }

    (Size::new(width, top + bottom), top)
}

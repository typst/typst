use std::borrow::Cow;
use std::ops::Range;

use rustybuzz::UnicodeBuffer;

use super::prelude::*;
use crate::font::{Face, FaceId, FontFamily, FontVariant};
use crate::geom::{Dir, Em, Length, Point, Size};
use crate::style::{Style, TextStyle};
use crate::util::SliceExt;

/// `font`: Configure the font.
pub fn font(ctx: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    struct FontDef(Rc<Vec<FontFamily>>);
    struct FamilyDef(Rc<Vec<String>>);

    castable! {
        FontDef: "font family or array of font families",
        Value::Str(string) => Self(Rc::new(vec![FontFamily::Named(string.to_lowercase())])),
        Value::Array(values) => Self(Rc::new(
            values
                .into_iter()
                .filter_map(|v| v.cast().ok())
                .collect()
        )),
        @family: FontFamily => Self(Rc::new(vec![family.clone()])),
    }

    castable! {
        FamilyDef: "string or array of strings",
        Value::Str(string) => Self(Rc::new(vec![string.to_lowercase()])),
        Value::Array(values) => Self(Rc::new(
            values
                .into_iter()
                .filter_map(|v| v.cast().ok())
                .map(|string: Str| string.to_lowercase())
                .collect()
        )),
    }

    let list = args.named("family")?.or_else(|| {
        let families: Vec<_> = args.all().collect();
        (!families.is_empty()).then(|| FontDef(Rc::new(families)))
    });

    let size = args.named::<Linear>("size")?.or_else(|| args.find());
    let style = args.named("style")?;
    let weight = args.named("weight")?;
    let stretch = args.named("stretch")?;
    let fill = args.named("fill")?.or_else(|| args.find());
    let top_edge = args.named("top-edge")?;
    let bottom_edge = args.named("bottom-edge")?;
    let serif = args.named("serif")?;
    let sans_serif = args.named("sans-serif")?;
    let monospace = args.named("monospace")?;
    let fallback = args.named("fallback")?;
    let body = args.find::<Template>();

    let f = move |style_: &mut Style| {
        let text = style_.text_mut();

        if let Some(size) = size {
            text.size = size.resolve(text.size);
        }

        if let Some(style) = style {
            text.variant.style = style;
        }

        if let Some(weight) = weight {
            text.variant.weight = weight;
        }

        if let Some(stretch) = stretch {
            text.variant.stretch = stretch;
        }

        if let Some(top_edge) = top_edge {
            text.top_edge = top_edge;
        }

        if let Some(bottom_edge) = bottom_edge {
            text.bottom_edge = bottom_edge;
        }

        if let Some(fill) = fill {
            text.fill = Paint::Color(fill);
        }

        if let Some(FontDef(list)) = &list {
            text.families_mut().list = list.clone();
        }

        if let Some(FamilyDef(serif)) = &serif {
            text.families_mut().serif = serif.clone();
        }

        if let Some(FamilyDef(sans_serif)) = &sans_serif {
            text.families_mut().sans_serif = sans_serif.clone();
        }

        if let Some(FamilyDef(monospace)) = &monospace {
            text.families_mut().monospace = monospace.clone();
        }

        if let Some(fallback) = fallback {
            text.fallback = fallback;
        }
    };

    Ok(if let Some(body) = body {
        Value::Template(body.modified(f))
    } else {
        ctx.template.modify(f);
        Value::None
    })
}

/// Shape text into [`ShapedText`].
pub fn shape<'a>(
    ctx: &mut LayoutContext,
    text: &'a str,
    style: &'a TextStyle,
    dir: Dir,
) -> ShapedText<'a> {
    let mut glyphs = vec![];
    if !text.is_empty() {
        shape_segment(
            ctx,
            &mut glyphs,
            0,
            text,
            style.size,
            style.variant(),
            style.families(),
            None,
            dir,
        );
    }

    let (size, baseline) = measure(ctx, &glyphs, style);
    ShapedText {
        text,
        dir,
        style,
        size,
        baseline,
        glyphs: Cow::Owned(glyphs),
    }
}

/// The result of shaping text.
///
/// This type contains owned or borrowed shaped text runs, which can be
/// measured, used to reshape substrings more quickly and converted into a
/// frame.
#[derive(Debug, Clone)]
pub struct ShapedText<'a> {
    /// The text that was shaped.
    pub text: &'a str,
    /// The text direction.
    pub dir: Dir,
    /// The properties used for font selection.
    pub style: &'a TextStyle,
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
}

impl<'a> ShapedText<'a> {
    /// Build the shaped text's frame.
    pub fn build(&self) -> Frame {
        let mut frame = Frame::new(self.size, self.baseline);
        let mut offset = Length::zero();

        for (face_id, group) in self.glyphs.as_ref().group_by_key(|g| g.face_id) {
            let pos = Point::new(offset, self.baseline);

            let mut text = Text {
                face_id,
                size: self.style.size,
                width: Length::zero(),
                fill: self.style.fill,
                glyphs: vec![],
            };

            for glyph in group {
                text.glyphs.push(Glyph {
                    id: glyph.glyph_id,
                    x_advance: glyph.x_advance,
                    x_offset: glyph.x_offset,
                });
                text.width += glyph.x_advance.to_length(text.size);
            }

            offset += text.width;
            frame.push(pos, Element::Text(text));
        }

        frame
    }

    /// Reshape a range of the shaped text, reusing information from this
    /// shaping process if possible.
    pub fn reshape(
        &'a self,
        ctx: &mut LayoutContext,
        text_range: Range<usize>,
    ) -> ShapedText<'a> {
        if let Some(glyphs) = self.slice_safe_to_break(text_range.clone()) {
            let (size, baseline) = measure(ctx, glyphs, self.style);
            Self {
                text: &self.text[text_range],
                dir: self.dir,
                style: self.style,
                size,
                baseline,
                glyphs: Cow::Borrowed(glyphs),
            }
        } else {
            shape(ctx, &self.text[text_range], self.style, self.dir)
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

/// A visual side.
enum Side {
    Left,
    Right,
}

/// Shape text with font fallback using the `families` iterator.
fn shape_segment<'a>(
    ctx: &mut LayoutContext,
    glyphs: &mut Vec<ShapedGlyph>,
    base: usize,
    text: &str,
    size: Length,
    variant: FontVariant,
    mut families: impl Iterator<Item = &'a str> + Clone,
    mut first_face: Option<FaceId>,
    dir: Dir,
) {
    // Select the font family.
    let (face_id, fallback) = loop {
        // Try to load the next available font family.
        match families.next() {
            Some(family) => {
                if let Some(id) = ctx.fonts.select(family, variant) {
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
    let mut face = ctx.fonts.get(face_id);
    let buffer = rustybuzz::shape(face.ttf(), &[], buffer);
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
                ctx,
                glyphs,
                base + range.start,
                &text[range],
                size,
                variant,
                families.clone(),
                first_face,
                dir,
            );

            face = ctx.fonts.get(face_id);
        }

        i += 1;
    }
}

/// Measure the size and baseline of a run of shaped glyphs with the given
/// properties.
fn measure(
    ctx: &mut LayoutContext,
    glyphs: &[ShapedGlyph],
    style: &TextStyle,
) -> (Size, Length) {
    let mut width = Length::zero();
    let mut top = Length::zero();
    let mut bottom = Length::zero();

    // Expand top and bottom by reading the face's vertical metrics.
    let mut expand = |face: &Face| {
        top.set_max(face.vertical_metric(style.top_edge, style.size));
        bottom.set_max(-face.vertical_metric(style.bottom_edge, style.size));
    };

    if glyphs.is_empty() {
        // When there are no glyphs, we just use the vertical metrics of the
        // first available font.
        for family in style.families() {
            if let Some(face_id) = ctx.fonts.select(family, style.variant) {
                expand(ctx.fonts.get(face_id));
                break;
            }
        }
    } else {
        for (face_id, group) in glyphs.group_by_key(|g| g.face_id) {
            let face = ctx.fonts.get(face_id);
            expand(face);

            for glyph in group {
                width += glyph.x_advance.to_length(style.size);
            }
        }
    }

    (Size::new(width, top + bottom), top)
}

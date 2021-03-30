use fontdock::FaceId;
use rustybuzz::UnicodeBuffer;
use ttf_parser::GlyphId;

use super::{Element, Frame, ShapedText};
use crate::env::FontLoader;
use crate::exec::FontProps;
use crate::geom::{Dir, Point, Size};

/// Shape text into a frame containing [`ShapedText`] runs.
pub fn shape(text: &str, dir: Dir, loader: &mut FontLoader, props: &FontProps) -> Frame {
    let mut frame = Frame::new(Size::ZERO);
    let iter = props.families.iter();
    shape_segment(&mut frame, text, dir, loader, props, iter, None);
    frame
}

/// Shape text into a frame with font fallback using the `families` iterator.
fn shape_segment<'a>(
    frame: &mut Frame,
    text: &str,
    dir: Dir,
    loader: &mut FontLoader,
    props: &FontProps,
    mut families: impl Iterator<Item = &'a str> + Clone,
    mut first: Option<FaceId>,
) {
    // Select the font family.
    let (id, fallback) = loop {
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
        first = Some(id);
    }

    // Find out some metrics and prepare the shaped text container.
    let face = loader.face(id);
    let ttf = face.ttf();
    let units_per_em = f64::from(ttf.units_per_em().unwrap_or(1000));
    let convert = |units| f64::from(units) / units_per_em * props.size;
    let top = convert(i32::from(props.top_edge.lookup(ttf)));
    let bottom = convert(i32::from(props.bottom_edge.lookup(ttf)));
    let mut shaped = ShapedText::new(id, props.size, top, bottom, props.color);

    // Fill the buffer with our text.
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(match dir {
        Dir::LTR => rustybuzz::Direction::LeftToRight,
        Dir::RTL => rustybuzz::Direction::RightToLeft,
        _ => unimplemented!(),
    });

    // Shape!
    let glyphs = rustybuzz::shape(face.buzz(), &[], buffer);
    let info = glyphs.glyph_infos();
    let pos = glyphs.glyph_positions();
    let mut iter = info.iter().zip(pos).peekable();

    while let Some((info, pos)) = iter.next() {
        // Do font fallback if the glyph is a tofu.
        if info.codepoint == 0 && fallback {
            // Flush what we have so far.
            if !shaped.glyphs.is_empty() {
                place(frame, shaped);
                shaped = ShapedText::new(id, props.size, top, bottom, props.color);
            }

            // Determine the start and end cluster index of the tofu sequence.
            let mut start = info.cluster as usize;
            let mut end = info.cluster as usize;
            while let Some((info, _)) = iter.peek() {
                if info.codepoint != 0 {
                    break;
                }
                end = info.cluster as usize;
                iter.next();
            }

            // Because Harfbuzz outputs glyphs in visual order, the start
            // cluster actually corresponds to the last codepoint in
            // right-to-left text.
            if !dir.is_positive() {
                assert!(end <= start);
                std::mem::swap(&mut start, &mut end);
            }

            // The end cluster index points right before the last character that
            // mapped to the tofu sequence. So we have to offset the end by one
            // char.
            let offset = text[end ..].chars().next().unwrap().len_utf8();
            let range = start .. end + offset;
            let part = &text[range];

            // Recursively shape the tofu sequence with the next family.
            shape_segment(frame, part, dir, loader, props, families.clone(), first);
        } else {
            // Add the glyph to the shaped output.
            // TODO: Don't ignore y_advance and y_offset.
            let glyph = GlyphId(info.codepoint as u16);
            shaped.glyphs.push(glyph);
            shaped.offsets.push(shaped.width + convert(pos.x_offset));
            shaped.width += convert(pos.x_advance);
        }
    }

    if !shaped.glyphs.is_empty() {
        place(frame, shaped)
    }
}

/// Place shaped text into a frame.
fn place(frame: &mut Frame, shaped: ShapedText) {
    let offset = frame.size.width;
    frame.size.width += shaped.width;
    frame.size.height = frame.size.height.max(shaped.top - shaped.bottom);
    frame.push(Point::new(offset, shaped.top), Element::Text(shaped));
}

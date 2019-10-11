//! Layouting of text into boxes.

use toddle::query::{FontQuery, SharedFontLoader};
use toddle::tables::{Header, CharMap, HorizontalMetrics};

use crate::size::{Size, Size2D};
use super::*;


/// The context for text layouting.
#[derive(Copy, Clone)]
pub struct TextContext<'a, 'p> {
    /// Loads fonts matching queries.
    pub loader: &'a SharedFontLoader<'p>,
    /// Base style to set text with.
    pub style: &'a TextStyle,
}

/// Layout one piece of text without any breaks as one continous box.
pub fn layout(text: &str, ctx: TextContext) -> LayoutResult<BoxLayout> {
    let mut loader = ctx.loader.borrow_mut();

    let mut actions = Vec::new();
    let mut active_font = std::usize::MAX;
    let mut buffer = String::new();
    let mut width = Size::zero();

    // Walk the characters.
    for character in text.chars() {
        // Retrieve the best font for this character.
        let mut font = None;
        let mut classes = ctx.style.classes.clone();
        for class in &ctx.style.fallback {
            classes.push(class.clone());

            font = loader.get(FontQuery {
                chars: &[character],
                classes: &classes,
            });

            if font.is_some() {
                break;
            }

            classes.pop();
        }

        let (font, index) = match font {
            Some(f) => f,
            None => return Err(LayoutError::NoSuitableFont(character)),
        };

        // Create a conversion function between font units and sizes.
        let font_unit_ratio = 1.0 / (font.read_table::<Header>()?.units_per_em as f32);
        let font_unit_to_size = |x| Size::pt(font_unit_ratio * x);

        // Add the char width to the total box width.
        let glyph = font.read_table::<CharMap>()?
            .get(character)
            .expect("layout text: font should have char");

        let glyph_width = font_unit_to_size(
            font.read_table::<HorizontalMetrics>()?
                .get(glyph)
                .expect("layout text: font should have glyph")
                .advance_width as f32
        );

        let char_width = glyph_width * ctx.style.font_size;
        width += char_width;

        // Change the font if necessary.
        if active_font != index {
            if !buffer.is_empty() {
                actions.push(LayoutAction::WriteText(buffer));
                buffer = String::new();
            }

            actions.push(LayoutAction::SetFont(index, ctx.style.font_size));
            active_font = index;
        }

        buffer.push(character);
    }

    // Write the remaining characters.
    if !buffer.is_empty() {
        actions.push(LayoutAction::WriteText(buffer));
    }

    Ok(BoxLayout {
        dimensions: Size2D::new(width, Size::pt(ctx.style.font_size)),
        actions,
        debug_render: false,
    })
}

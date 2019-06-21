//! Layouting of text into boxes.

use crate::doc::TextAction;
use crate::font::FontQuery;
use crate::size::{Size, Size2D};
use super::*;


/// The context for text layouting.
#[derive(Debug, Clone)]
pub struct TextContext<'a, 'p> {
    /// Loads fonts matching queries.
    pub loader: &'a FontLoader<'p>,
    /// Base style to set text with.
    pub style: TextStyle,
}

/// Layout one piece of text without any breaks as one continous box.
pub fn layout(text: &str, ctx: &TextContext) -> LayoutResult<BoxLayout> {
    let mut actions = Vec::new();
    let mut active_font = std::usize::MAX;
    let mut buffer = String::new();
    let mut width = Size::zero();

    // Walk the characters.
    for character in text.chars() {
        // Retrieve the best font for this character.
        let (index, font) = ctx.loader.get(FontQuery {
            families: ctx.style.font_families.clone(),
            italic: ctx.style.italic,
            bold: ctx.style.bold,
            character,
        }).ok_or_else(|| LayoutError::NoSuitableFont(character))?;

        // Add the char width to the total box width.
        let char_width = font.widths[font.map(character) as usize] * ctx.style.font_size;
        width += char_width;

        // Change the font if necessary.
        if active_font != index {
            if !buffer.is_empty() {
                actions.push(TextAction::WriteText(buffer));
                buffer = String::new();
            }

            actions.push(TextAction::SetFont(index, ctx.style.font_size));
            active_font = index;
        }

        buffer.push(character);
    }

    // Write the remaining characters.
    if !buffer.is_empty() {
        actions.push(TextAction::WriteText(buffer));
    }

    Ok(BoxLayout {
        dimensions: Size2D::new(width, Size::points(ctx.style.font_size)),
        actions,
    })
}

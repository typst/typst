//! Layouting of text into boxes.

use crate::doc::LayoutAction;
use crate::font::FontQuery;
use crate::size::{Size, Size2D};
use super::*;


/// The context for text layouting.
#[derive(Debug, Copy, Clone)]
pub struct TextContext<'a, 'p> {
    /// Loads fonts matching queries.
    pub loader: &'a FontLoader<'p>,
    /// Base style to set text with.
    pub style: &'a TextStyle,
}

/// Layout one piece of text without any breaks as one continous box.
pub fn layout(text: &str, ctx: TextContext) -> LayoutResult<BoxLayout> {
    let mut actions = Vec::new();
    let mut active_font = std::usize::MAX;
    let mut buffer = String::new();
    let mut width = Size::zero();

    // Walk the characters.
    for character in text.chars() {
        // Retrieve the best font for this character.
        let (index, font) = ctx.loader.get(FontQuery {
            classes: ctx.style.classes.clone(),
            fallback: ctx.style.fallback.clone(),
            character,
        }).ok_or_else(|| LayoutError::NoSuitableFont(character))?;

        // Add the char width to the total box width.
        let char_width = font.widths[font.encode(character) as usize] * ctx.style.font_size;
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
    })
}

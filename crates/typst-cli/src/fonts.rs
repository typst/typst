use std::path::Path;

use typst::text::FontVariant;
use typst_kit::fonts::Fonts;

use crate::args::FontsCommand;

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) {
    let mut fonts = Fonts::searcher();
    fonts.include_system_fonts(!command.font.ignore_system_fonts);
    #[cfg(feature = "embed-fonts")]
    fonts.include_embedded_fonts(!command.font.ignore_embedded_fonts);
    let fonts = fonts.search_with(&command.font.font_paths);

    for (family, indices) in fonts.book.families() {
        println!("{family}");
        if command.variants {
            for index in indices {
                let Some(font_info) = fonts.book.info(index) else { continue };
                let FontVariant { style, weight, stretch } = font_info.variant;
                let path = fonts
                    .slots
                    .get(index)
                    .and_then(|slot| slot.path())
                    .unwrap_or_else(|| Path::new("<embedded>"))
                    .display();

                println!(
                    "- Style: {style:?}, Weight: {weight:?}, Stretch: {stretch:?}, Path: {path}",
                );
            }
        }
    }
}

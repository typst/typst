use typst::text::FontVariant;
use typst_kit::fonts::{Fonts, IncludeFontsConfig};

use crate::args::{FontArgs, FontsCommand};

impl FontArgs {
    pub fn include_fonts_config(&self) -> IncludeFontsConfig {
        IncludeFontsConfig {
            include_system_fonts: !self.ignore_system_fonts,
            #[cfg(feature = "embed-fonts")]
            include_embedded_fonts: !self.ignore_embedded_fonts,
        }
    }
}

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) {
    let fonts = Fonts::searcher(command.font.include_fonts_config())
        .search_with(&command.font.font_paths);

    for (name, infos) in fonts.book.families() {
        println!("{name}");
        if command.variants {
            for info in infos {
                let FontVariant { style, weight, stretch } = info.variant;
                println!("- Style: {style:?}, Weight: {weight:?}, Stretch: {stretch:?}");
            }
        }
    }
}

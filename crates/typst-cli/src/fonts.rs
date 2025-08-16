use typst::text::FontVariantCoverage;
use typst_kit::fonts::Fonts;

use crate::args::FontsCommand;

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) {
    let fonts = Fonts::searcher()
        .include_system_fonts(!command.font.ignore_system_fonts)
        .search_with(&command.font.font_paths);

    for (name, infos) in fonts.book.families() {
        println!("{name}");
        if command.variants {
            for info in infos {
                let FontVariantCoverage { style, weight, stretch } = info.variant_coverage.clone();
                println!("- Style: {style:?}, Weight: {weight:?}, Stretch: {stretch:?}");
            }
        }
    }
}

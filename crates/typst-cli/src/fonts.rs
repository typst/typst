use typst::text::FontVariant;
use typst_kit::fonts::Fonts;

use crate::args::FontsCommand;

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) {
    let fonts = Fonts::searcher()
        .include_system_fonts(!command.font_args.ignore_system_fonts)
        .search_with(&command.font_args.font_paths);

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

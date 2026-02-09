use std::any::Any;
use std::path::Path;

use typst::text::FontVariant;
use typst_kit::fonts::{self, FontPath, FontStore};

use crate::args::{FontArgs, FontsCommand};

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) {
    let fonts = discover_fonts(&command.font);

    for (family, indices) in fonts.book().families() {
        println!("{family}");
        if command.variants {
            for index in indices {
                let Some(info) = fonts.book().info(index) else { continue };
                let FontVariant { style, weight, stretch } = info.variant;
                let path = fonts
                    .source(index)
                    .and_then(|source| (source as &dyn Any).downcast_ref::<FontPath>())
                    .map(|font| font.path.as_path())
                    .unwrap_or_else(|| Path::new("<embedded>"))
                    .display();

                println!(
                    "- Style: {style:?}, Weight: {weight:?}, Stretch: {stretch:?}, Path: {path}",
                );
            }
        }
    }
}

/// Discovers the fonts as specified by the CLI flags.
#[typst_macros::time(name = "discover fonts")]
pub fn discover_fonts(args: &FontArgs) -> FontStore {
    let mut fonts = FontStore::new();

    if !args.ignore_system_fonts {
        fonts.extend(fonts::system());
    }

    #[cfg(feature = "embedded-fonts")]
    if !args.ignore_embedded_fonts {
        fonts.extend(fonts::embedded());
    }

    for path in &args.font_paths {
        fonts.extend(fonts::scan(path));
    }

    fonts
}

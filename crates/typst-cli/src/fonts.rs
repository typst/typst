use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use fontdb::{Database, Source};
use typst::diag::StrResult;
use typst::text::{Font, FontBook, FontInfo, FontVariant};
use typst_assets_macro::include_asset;
use typst_timing::TimingScope;

use crate::args::FontsCommand;

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) -> StrResult<()> {
    let mut searcher = FontSearcher::new();
    searcher.search(&command.font_paths);

    for (name, infos) in searcher.book.families() {
        println!("{name}");
        if command.variants {
            for info in infos {
                let FontVariant { style, weight, stretch } = info.variant;
                println!("- Style: {style:?}, Weight: {weight:?}, Stretch: {stretch:?}");
            }
        }
    }

    Ok(())
}

/// Searches for fonts.
pub struct FontSearcher {
    /// Metadata about all discovered fonts.
    pub book: FontBook,
    /// Slots that the fonts are loaded into.
    pub fonts: Vec<FontSlot>,
}

/// Holds details about the location of a font and lazily the font itself.
pub struct FontSlot {
    /// The path at which the font can be found on the system.
    path: PathBuf,
    /// The index of the font in its collection. Zero if the path does not point
    /// to a collection.
    index: u32,
    /// The lazily loaded font.
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    /// Get the font for this slot.
    pub fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let _scope = TimingScope::new("load font", None);
                let data = fs::read(&self.path).ok()?.into();
                Font::new(data, self.index)
            })
            .clone()
    }
}

impl FontSearcher {
    /// Create a new, empty system searcher.
    pub fn new() -> Self {
        Self { book: FontBook::new(), fonts: vec![] }
    }

    /// Search everything that is available.
    pub fn search(&mut self, font_paths: &[PathBuf]) {
        let mut db = Database::new();

        // Font paths have highest priority.
        for path in font_paths {
            db.load_fonts_dir(path);
        }

        // System fonts have second priority.
        db.load_system_fonts();

        for face in db.faces() {
            let path = match &face.source {
                Source::File(path) | Source::SharedFile(path, _) => path,
                // We never add binary sources to the database, so there
                // shouln't be any.
                Source::Binary(_) => continue,
            };

            let info = db
                .with_face_data(face.id, FontInfo::new)
                .expect("database must contain this font");

            if let Some(info) = info {
                self.book.push(info);
                self.fonts.push(FontSlot {
                    path: path.clone(),
                    index: face.index,
                    font: OnceLock::new(),
                });
            }
        }

        // Embedded fonts have lowest priority.
        #[cfg(feature = "embed-fonts")]
        self.add_embedded();
    }

    /// Add fonts that are embedded in the binary.
    #[cfg(feature = "embed-fonts")]
    fn add_embedded(&mut self) {
        let mut process = |bytes: &'static [u8]| {
            let buffer = typst::foundations::Bytes::from_static(bytes);
            for (i, font) in Font::iter(buffer).enumerate() {
                self.book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    path: PathBuf::new(),
                    index: i as u32,
                    font: OnceLock::from(Some(font)),
                });
            }
        };

        // Embed default fonts.
        process(include_asset!("LinLibertine_R.ttf"));
        process(include_asset!("LinLibertine_RB.ttf"));
        process(include_asset!("LinLibertine_RBI.ttf"));
        process(include_asset!("LinLibertine_RI.ttf"));
        process(include_asset!("NewCMMath-Book.otf"));
        process(include_asset!("NewCMMath-Regular.otf"));
        process(include_asset!("NewCM10-Bold.otf"));
        process(include_asset!("NewCM10-BoldItalic.otf"));
        process(include_asset!("NewCM10-Italic.otf"));
        process(include_asset!("NewCM10-Regular.otf"));
        process(include_asset!("DejaVuSansMono-Bold.ttf"));
        process(include_asset!("DejaVuSansMono-BoldOblique.ttf"));
        process(include_asset!("DejaVuSansMono-Oblique.ttf"));
        process(include_asset!("DejaVuSansMono.ttf"));
    }
}

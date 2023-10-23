use std::cell::OnceCell;
use std::fs;
use std::path::{Path, PathBuf};

use fontdb::{Database, Source};
use typst::diag::StrResult;
use typst::font::{Font, FontBook, FontInfo, FontVariant};

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
    /// Font database of fontdb crate used to search fonts.
    pub db: Database,
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
    font: OnceCell<Option<Font>>,
}

impl FontSlot {
    /// Get the font for this slot.
    pub fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = fs::read(&self.path).ok()?.into();
                Font::new(data, self.index)
            })
            .clone()
    }
}

impl FontSearcher {
    /// Create a new, empty system searcher.
    pub fn new() -> Self {
        Self {
            db: Database::new(),
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    /// Search everything that is available.
    pub fn search(&mut self, font_paths: &[PathBuf]) {
        for path in font_paths {
            self.search_dir(path)
        }

        self.search_system();

        #[cfg(feature = "embed-fonts")]
        self.add_embedded();

        for face in self.db.faces() {
            let (path, info) = match face.source {
                Source::File(ref path) | Source::SharedFile(ref path, _) => {
                    let info = self
                        .db
                        .with_face_data(face.id, FontInfo::new)
                        .expect("face got from the same database, call with_face_data with it's id must not None");

                    (path, info)
                }
                Source::Binary(_) => continue, // already processed when we add it
            };
            if let Some(info) = info {
                self.book.push(info);
                self.fonts.push(FontSlot {
                    path: path.clone(),
                    index: face.index,
                    font: OnceCell::new(),
                });
            }
        }
    }

    /// Add fonts that are embedded in the binary.
    #[cfg(feature = "embed-fonts")]
    fn add_embedded(&mut self) {
        let mut process = |bytes: &'static [u8]| {
            let buffer = typst::eval::Bytes::from_static(bytes);
            for (i, font) in Font::iter(buffer).enumerate() {
                self.book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    path: PathBuf::new(),
                    index: i as u32,
                    font: OnceCell::from(Some(font)),
                });
            }
        };

        macro_rules! add {
            ($filename:literal) => {
                process(include_bytes!(concat!("../../../assets/fonts/", $filename)));
            };
        }

        // Embed default fonts.
        add!("LinLibertine_R.ttf");
        add!("LinLibertine_RB.ttf");
        add!("LinLibertine_RBI.ttf");
        add!("LinLibertine_RI.ttf");
        add!("NewCMMath-Book.otf");
        add!("NewCMMath-Regular.otf");
        add!("NewCM10-Regular.otf");
        add!("NewCM10-Bold.otf");
        add!("NewCM10-Italic.otf");
        add!("NewCM10-BoldItalic.otf");
        add!("DejaVuSansMono.ttf");
        add!("DejaVuSansMono-Bold.ttf");
        add!("DejaVuSansMono-Oblique.ttf");
        add!("DejaVuSansMono-BoldOblique.ttf");
    }

    /// Search for fonts in the linux system font directories.
    fn search_system(&mut self) {
        self.db.load_system_fonts()
    }

    /// Search for all fonts in a directory recursively.
    fn search_dir(&mut self, path: impl AsRef<Path>) {
        self.db.load_fonts_dir(path)
    }

    /// Index the fonts in the file at the given path.
    #[allow(dead_code)] // Keep this in case we need to support adding a single font file
    fn search_file(&mut self, path: &Path) {
        let _ = self.db.load_font_file(path);
    }
}

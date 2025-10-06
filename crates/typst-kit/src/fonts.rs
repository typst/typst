//! Default implementation for searching local and system installed fonts as
//! well as loading embedded default fonts.
//!
//! # Embedded fonts
//! The following fonts are available as embedded fonts via the `embed-fonts`
//! feature flag:
//! - For text: Libertinus Serif, New Computer Modern
//! - For math: New Computer Modern Math
//! - For code: Deja Vu Sans Mono

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use fontdb::{Database, Source};
use typst_library::foundations::Bytes;
use typst_library::text::{Font, FontBook, FontInfo};
use typst_timing::TimingScope;

/// Holds details about the location of a font and lazily the font itself.
#[derive(Debug)]
pub struct FontSlot {
    /// The path at which the font can be found on the system.
    path: Option<PathBuf>,
    /// The index of the font in its collection. Zero if the path does not point
    /// to a collection.
    index: u32,
    /// The lazily loaded font.
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    /// Returns the path at which the font can be found on the system, or `None`
    /// if the font was embedded.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the index of the font in its collection. Zero if the path does
    /// not point to a collection.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the font for this slot. This loads the font into memory on first
    /// access.
    pub fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let _scope = TimingScope::new("load font");
                let data = fs::read(
                    self.path
                        .as_ref()
                        .expect("`path` is not `None` if `font` is uninitialized"),
                )
                .ok()?;
                Font::new(Bytes::new(data), self.index)
            })
            .clone()
    }
}

/// The result of a font search, created by calling [`FontSearcher::search`].
#[derive(Debug)]
pub struct Fonts {
    /// Metadata about all discovered fonts.
    pub book: FontBook,
    /// Slots that the fonts are loaded into.
    pub fonts: Vec<FontSlot>,
}

impl Fonts {
    /// Creates a new font searcher with the default settings.
    pub fn searcher(include_config: IncludeFontsConfig) -> FontSearcher {
        FontSearcher::new(include_config)
    }
}

#[derive(Debug)]
pub struct IncludeFontsConfig {
    pub include_system_fonts: bool,
    #[cfg(feature = "embed-fonts")]
    pub include_embedded_fonts: bool,
}

impl Default for IncludeFontsConfig {
    fn default() -> Self {
        Self {
            include_system_fonts: true,
            include_embedded_fonts: true,
        }
    }
}

/// Searches for fonts.
///
/// Fonts are added in the following order (descending priority):
/// 1. Font directories
/// 2. System fonts (if included & enabled)
/// 3. Embedded fonts (if enabled)
#[derive(Debug)]
pub struct FontSearcher {
    db: Database,
    include_config: IncludeFontsConfig,
    book: FontBook,
    fonts: Vec<FontSlot>,
}

impl FontSearcher {
    /// Create a new, empty system searcher. The searcher is created with the
    /// default configuration, it will include embedded fonts and system fonts.
    pub fn new(include_config: IncludeFontsConfig) -> Self {
        Self {
            db: Database::new(),
            include_config,
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    /// Whether to search for and load system fonts, defaults to `true`.
    pub fn include_system_fonts(&mut self, value: bool) -> &mut Self {
        self.include_config.include_system_fonts = value;
        self
    }

    /// Whether to load embedded fonts, defaults to `true`.
    #[cfg(feature = "embed-fonts")]
    pub fn include_embedded_fonts(&mut self, value: bool) -> &mut Self {
        self.include_config.include_embedded_fonts = value;
        self
    }

    /// Start searching for and loading fonts. To additionally load fonts
    /// from specific directories, use [`search_with`][Self::search_with].
    ///
    /// # Examples
    /// ```no_run
    /// # use typst_kit::fonts::FontSearcher;
    /// let fonts = FontSearcher::new()
    ///     .include_system_fonts(true)
    ///     .search();
    /// ```
    pub fn search(&mut self) -> Fonts {
        self.search_with::<_, &str>([])
    }

    /// Start searching for and loading fonts, with additional directories.
    ///
    /// # Examples
    /// ```no_run
    /// # use typst_kit::fonts::FontSearcher;
    /// let fonts = FontSearcher::new()
    ///     .include_system_fonts(true)
    ///     .search_with(["./assets/fonts/"]);
    /// ```
    pub fn search_with<I, P>(&mut self, font_dirs: I) -> Fonts
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        // Font paths have highest priority.
        for path in font_dirs {
            self.db.load_fonts_dir(path);
        }

        if self.include_config.include_system_fonts {
            // System fonts have second priority.
            self.db.load_system_fonts();
        }

        for face in self.db.faces() {
            let path = match &face.source {
                Source::File(path) | Source::SharedFile(path, _) => path,
                // We never add binary sources to the database, so there
                // shouldn't be any.
                Source::Binary(_) => continue,
            };

            let info = self
                .db
                .with_face_data(face.id, FontInfo::new)
                .expect("database must contain this font");

            if let Some(info) = info {
                self.book.push(info);
                self.fonts.push(FontSlot {
                    path: Some(path.clone()),
                    index: face.index,
                    font: OnceLock::new(),
                });
            }
        }

        // Embedded fonts have lowest priority.
        #[cfg(feature = "embed-fonts")]
        if self.include_config.include_embedded_fonts {
            self.add_embedded();
        }

        Fonts {
            book: std::mem::take(&mut self.book),
            fonts: std::mem::take(&mut self.fonts),
        }
    }

    /// Add fonts that are embedded in the binary.
    #[cfg(feature = "embed-fonts")]
    fn add_embedded(&mut self) {
        for data in typst_assets::fonts() {
            let buffer = Bytes::new(data);
            for (i, font) in Font::iter(buffer).enumerate() {
                self.book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    path: None,
                    index: i as u32,
                    font: OnceLock::from(Some(font)),
                });
            }
        }
    }
}

impl Default for FontSearcher {
    fn default() -> Self {
        Self::new(IncludeFontsConfig::default())
    }
}

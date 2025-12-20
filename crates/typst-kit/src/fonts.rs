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
use typst_utils::LazyHash;

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
    ///
    /// Can directly be used in [`World::book`](typst_library::World::book).
    pub book: LazyHash<FontBook>,
    /// Slots that the fonts are loaded into.
    ///
    /// Assuming your world implementation has a field `fonts: Fonts`, this can
    /// be used in [`World::font`](typst_library::World::font) as such:
    /// ```ignore
    /// fn font(&self, index: usize) -> Option<Font> {
    ///     self.fonts.slots.get(index)?.get()
    /// }
    /// ```
    pub slots: Vec<FontSlot>,
}

/// A single variant together with an optional filesystem path where the
/// variant's font file is located. `path` is `None` for embedded fonts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariantSlot {
    pub info: FontInfo,
    pub path: Option<PathBuf>,
}

/// A family and its variants with optional slot paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FamilyWithSlots {
    pub family: String,
    pub variants: Vec<VariantSlot>,
}

impl Fonts {
    /// Creates a new font searcer with the default settings.
    pub fn searcher() -> FontSearcher {
        FontSearcher::new()
    }

    /// Returns an owned collection of all font families together with the
    /// corresponding `FontInfo` and an optional `PathBuf` where the font can
    /// be found.
    pub fn families_with_slots(&self) -> Vec<FamilyWithSlots> {
        let mut res = Vec::new();

        for (name, _) in self.book.families() {
            let mut variants = Vec::new();
            for id in self.book.select_family(&name.to_lowercase()) {
                if let Some(info) = self.book.info(id) {
                    let path = self
                        .slots
                        .get(id)
                        .and_then(|slot| slot.path().map(|p| p.to_path_buf()));
                    variants.push(VariantSlot { info: info.clone(), path });
                }
            }
            res.push(FamilyWithSlots { family: name.to_string(), variants });
        }

        res
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
    include_system_fonts: bool,
    #[cfg(feature = "embed-fonts")]
    include_embedded_fonts: bool,
    book: FontBook,
    fonts: Vec<FontSlot>,
}

impl FontSearcher {
    /// Create a new, empty system searcher. The searcher is created with the
    /// default configuration, it will include embedded fonts and system fonts.
    pub fn new() -> Self {
        Self {
            db: Database::new(),
            include_system_fonts: true,
            #[cfg(feature = "embed-fonts")]
            include_embedded_fonts: true,
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    /// Whether to search for and load system fonts, defaults to `true`.
    pub fn include_system_fonts(&mut self, value: bool) -> &mut Self {
        self.include_system_fonts = value;
        self
    }

    /// Whether to load embedded fonts, defaults to `true`.
    #[cfg(feature = "embed-fonts")]
    pub fn include_embedded_fonts(&mut self, value: bool) -> &mut Self {
        self.include_embedded_fonts = value;
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

        if self.include_system_fonts {
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
        if self.include_embedded_fonts {
            self.add_embedded();
        }

        Fonts {
            book: LazyHash::new(std::mem::take(&mut self.book)),
            slots: std::mem::take(&mut self.fonts),
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
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use typst_library::text::{
        Coverage, FontFlags, FontStretch, FontStyle, FontVariant, FontWeight,
    };
    use typst_utils::LazyHash;

    #[test]
    fn families_with_slots_returns_paths() {
        let mut book = FontBook::new();

        let info1 = FontInfo {
            family: "TestFamily".to_string(),
            variant: FontVariant::new(
                FontStyle::Normal,
                FontWeight::REGULAR,
                FontStretch::NORMAL,
            ),
            flags: FontFlags::empty(),
            coverage: Coverage::from_vec(vec![]),
        };

        let info2 = FontInfo {
            family: "TestFamily".to_string(),
            variant: FontVariant::new(
                FontStyle::Italic,
                FontWeight::BOLD,
                FontStretch::NORMAL,
            ),
            flags: FontFlags::empty(),
            coverage: Coverage::from_vec(vec![]),
        };

        book.push(info1.clone());
        book.push(info2.clone());

        let slots = vec![
            FontSlot {
                path: Some(PathBuf::from("/tmp/test-a.ttf")),
                index: 0,
                font: OnceLock::new(),
            },
            FontSlot { path: None, index: 0, font: OnceLock::new() },
        ];

        let fonts = Fonts { book: LazyHash::new(book), slots };

        let families = fonts.families_with_slots();
        assert_eq!(families.len(), 1);
        assert_eq!(families[0].family, "TestFamily");
        assert_eq!(families[0].variants.len(), 2);
        assert_eq!(
            families[0].variants[0].path.as_ref().and_then(|p| p.to_str()),
            Some("/tmp/test-a.ttf")
        );
        assert!(families[0].variants[1].path.is_none());
    }
}

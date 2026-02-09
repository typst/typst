//! Font loading and management.
//!
//! This provides implementations to discover fonts [in directories](scan) and
//! [from system](system) and can also serve standard [embedded] fonts.

use std::any::Any;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use typst_library::foundations::Bytes;
use typst_library::text::{Font, FontBook, FontInfo};
use typst_utils::LazyHash;

/// Holds loaded fonts.
///
/// Fonts can be added with [`push`](Self::push) and [`extend`](Self::extend).
/// The three top-level font provider functions in this module can directly be
/// used with [`FontStore::extend`].
///
/// Font are added in-order. The indices in the font book and those that should
/// be passed to [`source`](Self::source) and [`source`](Self::font) match this
/// order.
pub struct FontStore {
    book: LazyHash<FontBook>,
    slots: Vec<FontSlot>,
}

impl FontStore {
    /// Creates a new empty font store.
    pub fn new() -> Self {
        Self {
            book: LazyHash::new(FontBook::new()),
            slots: Vec::new(),
        }
    }

    /// Adds a new entry to the store.
    pub fn push(&mut self, entry: (impl FontSource, FontInfo)) {
        self.book.push(entry.1);
        self.slots
            .push(FontSlot { source: Box::new(entry.0), font: OnceLock::new() });
    }

    /// Adds multiple new entries to the store.
    pub fn extend<T>(&mut self, entries: impl IntoIterator<Item = (T, FontInfo)>)
    where
        T: FontSource,
    {
        for entry in entries {
            self.push(entry);
        }
    }

    /// Provides metadata for the added fonts.
    ///
    /// Can directly be used to implement
    /// [`World::book`](typst_library::World::book).
    pub fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    /// Retrieves the font at the given index.
    ///
    /// Loads the font if it's not already loaded.
    ///
    /// Can directly be used to implement
    /// [`World::font`](typst_library::World::font).
    pub fn font(&self, index: usize) -> Option<Font> {
        self.slots.get(index)?.get()
    }

    /// Retrieves the underlying font source for the font with this index.
    pub fn source(&self, index: usize) -> Option<&dyn FontSource> {
        Some(&*self.slots.get(index)?.source)
    }
}

impl Default for FontStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Holds a font source and the lazily loaded font itself.
struct FontSlot {
    source: Box<dyn FontSource>,
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    /// Get the font for this slot. This loads the font into memory on first
    /// access.
    fn get(&self) -> Option<Font> {
        self.font.get_or_init(|| self.source.load()).clone()
    }
}

/// Serves a font on-demand.
pub trait FontSource: Send + Sync + Any {
    /// Try to load the font.
    fn load(&self) -> Option<Font>;
}

impl FontSource for Font {
    fn load(&self) -> Option<Font> {
        Some(self.clone())
    }
}

impl FontSource for FontPath {
    fn load(&self) -> Option<Font> {
        let _scope = typst_timing::TimingScope::new("load font");
        let data = fs::read(&self.path).ok()?;
        Font::new(Bytes::new(data), self.index)
    }
}

/// Locates a font on the file system.
#[derive(Debug)]
pub struct FontPath {
    /// The path at which the font or font collection resides.
    pub path: PathBuf,
    /// The index in the font collection, or zero if the path points to a single
    /// font rather than a collection.
    pub index: u32,
}

/// Yields the embedded fonts.
///
/// - For Text: _Libertinus Serif_, _New Computer Modern_
/// - For Math: _New Computer Modern Math_
/// - For Code: _Deja Vu Sans Mono_
#[cfg(feature = "embedded-fonts")]
pub fn embedded() -> impl Iterator<Item = (Font, FontInfo)> {
    typst_assets::fonts().flat_map(|data| {
        Font::iter(Bytes::new(data)).map(|font| {
            let info = font.info().clone();
            (font, info)
        })
    })
}

/// Discovers system fonts.
///
/// This searches in operating-system dependant standard font locations.
#[cfg(feature = "scan-fonts")]
pub fn system() -> impl Iterator<Item = (FontPath, FontInfo)> {
    let _scope = typst_timing::TimingScope::new("scan system fonts");
    with_db(|db| {
        db.load_system_fonts();

        // Add Adobe Fonts on Windows and macOS.
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        load_adobe_fonts(db);
    })
}

/// Scans for fonts in a directory.
///
/// The directory is searched recursively.
#[cfg(feature = "scan-fonts")]
pub fn scan(path: &std::path::Path) -> impl Iterator<Item = (FontPath, FontInfo)> {
    let _scope = typst_timing::TimingScope::new("scan system fonts");
    with_db(move |db| db.load_fonts_dir(path))
}

/// Discovers fonts via `fontdb`.
#[cfg(feature = "scan-fonts")]
fn with_db(
    f: impl FnOnce(&mut fontdb::Database),
) -> impl Iterator<Item = (FontPath, FontInfo)> {
    let mut db = fontdb::Database::new();
    f(&mut db);
    db.faces()
        .filter_map(|face| {
            let path = match &face.source {
                fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => path,
                // We never add binary sources to the database, so there
                // shouldn't be any.
                fontdb::Source::Binary(_) => return None,
            };

            let info = db
                .with_face_data(face.id, FontInfo::new)
                .expect("database must contain this font")?;

            let path = FontPath { path: path.clone(), index: face.index };

            Some((path, info))
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// Loads Adobe fonts available on the system. Only supported on Windows and
/// macOS.
///
/// This is permissible as per Clause 3.1 (A) of the
/// [Adobe Fonts Service Product Specific Terms][terms].
///
/// [terms]: https://wwwimages2.adobe.com/content/dam/cc/en/legal/servicetou/Adobe-Fonts-Product-Specific-Terms-en_US-20241007.pdf
#[cfg(all(feature = "scan-fonts", any(target_os = "windows", target_os = "macos")))]
fn load_adobe_fonts(db: &mut fontdb::Database) {
    let Some(data) = dirs::data_dir() else { return };
    let base = data.join("Adobe");

    let prefix = if cfg!(target_os = "macos") { "." } else { "" };
    let subdirs = [
        format!("CoreSync/plugins/livetype/{prefix}r"),
        format!("{prefix}User Owned Fonts"),
    ];

    for subdir in subdirs {
        let Ok(entries) = fs::read_dir(base.join(subdir)) else { return };
        for entry in entries.flatten() {
            // Adobe fonts are stored as files (directories are skipped).
            let Ok(metadata) = entry.metadata() else { continue };
            if metadata.is_file() {
                db.load_font_file(entry.path()).ok();
            }
        }
    }
}

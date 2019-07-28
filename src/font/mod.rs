//! Font loading and subsetting.
//!
//! # Font handling
//! To do the typesetting, the engine needs font data. However, to be highly portable the engine
//! itself assumes nothing about the environment. To still work with fonts, the consumer of this
//! library has to add _font providers_ to their typesetting instance. These can be queried for font
//! data given flexible font filters specifying required font families and styles. A font provider
//! is a type implementing the [`FontProvider`](crate::font::FontProvider) trait.
//!
//! There is one [included font provider](crate::font::FileSystemFontProvider) that serves fonts
//! from a folder on the file system.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek, BufReader};
use std::path::{Path, PathBuf};

use opentype::{Error as OpentypeError, OpenTypeReader};
use opentype::tables::{Header, Name, CharMap, HorizontalMetrics, Post, OS2};
use opentype::types::{MacStyleFlags, NameEntry};
use toml::map::Map as TomlMap;
use toml::value::Value as TomlValue;

use self::subset::Subsetter;
use crate::size::Size;

mod loader;
mod subset;

pub use loader::{FontLoader, FontQuery};


/// A parsed _OpenType_ font program.
#[derive(Debug, Clone)]
pub struct Font {
    /// The name of the font.
    pub name: String,
    /// The complete, raw bytes of the font program.
    pub program: Vec<u8>,
    /// The mapping from character codes to glyph ids.
    pub mapping: HashMap<char, u16>,
    /// The widths of the glyphs indexed by glyph id.
    pub widths: Vec<Size>,
    /// The id of the fallback glyph.
    pub default_glyph: u16,
    /// The typesetting or exporting-relevant metrics of this font.
    pub metrics: FontMetrics,
}

/// Font metrics relevant to the typesetting or exporting processes.
#[derive(Debug, Copy, Clone)]
pub struct FontMetrics {
    /// Whether the font is italic.
    pub italic: bool,
    /// Whether font is monospace.
    pub monospace: bool,
    /// The angle of text in italics (in counter-clockwise degrees from vertical).
    pub italic_angle: f32,
    /// The extremal values [x_min, y_min, x_max, y_max] for all glyph bounding boxes.
    pub bounding_box: [Size; 4],
    /// The typographic ascender.
    pub ascender: Size,
    /// The typographic descender.
    pub descender: Size,
    /// The approximate height of capital letters.
    pub cap_height: Size,
    /// The weight class of the font (from 100 for thin to 900 for heavy).
    pub weight_class: u16,
}

impl Font {
    /// Create a `Font` from a raw font program.
    pub fn new(program: Vec<u8>) -> FontResult<Font> {
        let cursor = Cursor::new(&program);
        let mut reader = OpenTypeReader::new(cursor);

        // All of these tables are required by the OpenType specification,
        // so we do not really have to handle the case that they are missing.
        let head = reader.read_table::<Header>()?;
        let name = reader.read_table::<Name>()?;
        let os2  = reader.read_table::<OS2>()?;
        let cmap = reader.read_table::<CharMap>()?;
        let hmtx = reader.read_table::<HorizontalMetrics>()?;
        let post = reader.read_table::<Post>()?;

        // Create a conversion function between font units and sizes.
        let font_unit_ratio = 1.0 / (head.units_per_em as f32);
        let font_unit_to_size = |x| Size::pt(font_unit_ratio * x);

        let font_name = name
            .get_decoded(NameEntry::PostScriptName)
            .unwrap_or_else(|| "unknown".to_owned());

        let widths = hmtx.metrics.iter()
            .map(|m| font_unit_to_size(m.advance_width as f32)).collect();

        let metrics = FontMetrics {
            italic: head.mac_style.contains(MacStyleFlags::ITALIC),
            monospace: post.is_fixed_pitch,
            italic_angle: post.italic_angle.to_f32(),
            bounding_box: [
                font_unit_to_size(head.x_min as f32),
                font_unit_to_size(head.y_min as f32),
                font_unit_to_size(head.x_max as f32),
                font_unit_to_size(head.y_max as f32),
            ],
            ascender: font_unit_to_size(os2.s_typo_ascender as f32),
            descender: font_unit_to_size(os2.s_typo_descender as f32),
            cap_height: font_unit_to_size(os2.s_cap_height.unwrap_or(os2.s_typo_ascender) as f32),
            weight_class: os2.us_weight_class,
        };

        Ok(Font {
            name: font_name,
            program,
            mapping: cmap.mapping,
            widths,
            default_glyph: os2.us_default_char.unwrap_or(0),
            metrics,
        })
    }

    /// Encode a character into it's glyph id.
    #[inline]
    pub fn encode(&self, character: char) -> u16 {
        self.mapping.get(&character).map(|&g| g).unwrap_or(self.default_glyph)
    }

    /// Encode the given text into a vector of glyph ids.
    #[inline]
    pub fn encode_text(&self, text: &str) -> Vec<u8> {
        const BYTES_PER_GLYPH: usize = 2;
        let mut bytes = Vec::with_capacity(BYTES_PER_GLYPH * text.len());
        for c in text.chars() {
            let glyph = self.encode(c);
            bytes.push((glyph >> 8) as u8);
            bytes.push((glyph & 0xff) as u8);
        }
        bytes
    }

    /// Generate a subsetted version of this font.
    ///
    /// This version includes only the given `chars` and _OpenType_ `tables`.
    #[inline]
    pub fn subsetted<C, I, S>(&self, chars: C, tables: I) -> Result<Font, FontError>
    where
        C: IntoIterator<Item=char>,
        I: IntoIterator<Item=S>,
        S: AsRef<str>
    {
        Subsetter::subset(self, chars, tables)
    }
}

/// A type that provides fonts.
pub trait FontProvider {
    /// Returns a font with the given info if this provider has one.
    fn get(&self, info: &FontInfo) -> Option<Box<dyn FontData>>;

    /// The available fonts this provider can serve. While these should generally
    /// be retrievable through the `get` method, this is not guaranteed.
    fn available<'p>(&'p self) -> &'p [FontInfo];
}

/// A wrapper trait around `Read + Seek`.
///
/// This type is needed because currently you can't make a trait object with two traits, like
/// `Box<dyn Read + Seek>`. Automatically implemented for all types that are [`Read`] and [`Seek`].
pub trait FontData: Read + Seek {}
impl<T> FontData for T where T: Read + Seek {}

/// Classifies a font by listing the font classes it is part of.
///
/// All fonts with the same [`FontInfo`] are part of the same intersection
/// of [font classes](FontClass).
///
/// This structure can be constructed conveniently through the [`font`] macro.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontInfo {
    /// The font classes this font is part of.
    pub classes: Vec<FontClass>,
}

impl FontInfo {
    /// Create a new font info from a collection of classes.
    #[inline]
    pub fn new<I>(classes: I) -> FontInfo where I: IntoIterator<Item=FontClass> {
        FontInfo {
            classes: classes.into_iter().collect()
        }
    }
}

/// A class of fonts.
///
/// The set of all fonts can be classified into subsets of font classes like
/// _serif_ or _bold_. This enum lists such subclasses.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FontClass {
    Serif,
    SansSerif,
    Monospace,
    Regular,
    Bold,
    Italic,
    /// A custom family like _Arial_ or _Times_.
    Family(String),
}

/// A macro to create [FontInfos](crate::font::FontInfo) easily.
///
/// Accepts an ordered list of font classes. Strings expressions are parsed
/// into custom `Family`-variants and others can be named directly.
///
/// # Examples
/// ```
/// # use typeset::font;
/// // Noto Sans in regular typeface.
/// font!["NotoSans", "Noto", Regular, SansSerif];
///
/// // Noto Serif in italics and boldface.
/// font!["NotoSerif", "Noto", Bold, Italic, Serif];
///
/// // Arial in italics.
/// font!["Arial", Italic, SansSerif];
///
/// // Noto Emoji, which works in sans-serif and serif contexts.
/// font!["NotoEmoji", "Noto", Regular, SansSerif, Serif, Monospace];
/// ```
#[macro_export]
macro_rules! font {
    // Parse class list one by one.
    (@__cls $v:expr) => {};
    (@__cls $v:expr, $c:ident) => { $v.push($crate::font::FontClass::$c); };
    (@__cls $v:expr, $c:ident, $($tts:tt)*) => {
        font!(@__cls $v, $c);
        font!(@__cls $v, $($tts)*)
    };
    (@__cls $v:expr, $f:expr) => { $v.push( $crate::font::FontClass::Family($f.to_string())); };
    (@__cls $v:expr, $f:expr, $($tts:tt)*) => {
        font!(@__cls $v, $f);
        font!(@__cls $v, $($tts)*)
    };

    // Entry point
    ($($tts:tt)*) => {{
        let mut classes = Vec::new();
        font!(@__cls classes, $($tts)*);
        $crate::font::FontInfo { classes }
    }};
}

/// A font provider serving fonts from a folder on the local file system.
#[derive(Debug)]
pub struct FileSystemFontProvider {
    /// The base folder all other paths are relative to.
    base: PathBuf,
    /// Paths of the fonts relative to the `base` path.
    paths: Vec<PathBuf>,
    /// The info for the font with the same index in `paths`.
    infos: Vec<FontInfo>,
}

impl FileSystemFontProvider {
    /// Create a new provider serving fonts from a base path. The `fonts` iterator
    /// should contain paths of fonts relative to the base alongside matching
    /// infos for these fonts.
    ///
    /// # Example
    /// Serve the two fonts `NotoSans-Regular` and `NotoSans-Italic` from the local folder
    /// `../fonts`.
    /// ```
    /// # use typeset::{font::FileSystemFontProvider, font};
    /// FileSystemFontProvider::new("../fonts", vec![
    ///     ("NotoSans-Regular.ttf", font!["NotoSans", Regular, SansSerif]),
    ///     ("NotoSans-Italic.ttf", font!["NotoSans", Italic, SansSerif]),
    /// ]);
    /// ```
    pub fn new<B, I, P>(base: B, fonts: I) -> FileSystemFontProvider
    where
        B: Into<PathBuf>,
        I: IntoIterator<Item = (P, FontInfo)>,
        P: Into<PathBuf>,
    {
        let iter = fonts.into_iter();

        // Find out how long the iterator is at least, to reserve the correct
        // capacity for the vectors.
        let min = iter.size_hint().0;
        let mut paths = Vec::with_capacity(min);
        let mut infos = Vec::with_capacity(min);

        for (path, info) in iter {
            paths.push(path.into());
            infos.push(info);
        }

        FileSystemFontProvider {
            base: base.into(),
            paths,
            infos,
        }
    }

    /// Create a new provider from a font listing file.
    pub fn from_listing<P: AsRef<Path>>(file: P) -> FontResult<FileSystemFontProvider> {
        fn inv<S: ToString>(message: S) -> FontError {
            FontError::InvalidListing(message.to_string())
        }

        let file = file.as_ref();
        let base = file.parent()
            .ok_or_else(|| inv("expected listings file"))?;

        let bytes = fs::read(file)?;
        let map: TomlMap<String, toml::Value> = toml::de::from_slice(&bytes)
            .map_err(|err| inv(err))?;

        let mut paths = Vec::new();
        let mut infos = Vec::new();

        for value in map.values() {
            if let TomlValue::Table(table) = value {
                // Parse the string file key.
                paths.push(match table.get("file") {
                    Some(TomlValue::String(s)) => PathBuf::from(s),
                    _ => return Err(inv("expected file name")),
                });

                // Parse the array<string> classes key.
                infos.push(if let Some(TomlValue::Array(array)) = table.get("classes") {
                    let mut classes = Vec::with_capacity(array.len());
                    for class in array {
                        classes.push(match class {
                            TomlValue::String(class) => match class.as_str() {
                                "Serif" => FontClass::Serif,
                                "SansSerif" => FontClass::SansSerif,
                                "Monospace" => FontClass::Monospace,
                                "Regular" => FontClass::Regular,
                                "Bold" => FontClass::Bold,
                                "Italic" => FontClass::Italic,
                                _ => FontClass::Family(class.to_string()),
                            },
                            _ => return Err(inv("expect font class string")),
                        })
                    }
                    FontInfo { classes }
                } else {
                    return Err(inv("expected font classes"));
                });
            } else {
                return Err(inv("expected file/classes table"));
            }
        }

        Ok(FileSystemFontProvider {
            base: base.to_owned(),
            paths,
            infos,
        })
    }
}

impl FontProvider for FileSystemFontProvider {
    #[inline]
    fn get(&self, info: &FontInfo) -> Option<Box<dyn FontData>> {
        let index = self.infos.iter().position(|c| c == info)?;
        let path = &self.paths[index];
        let full_path = self.base.join(path);
        let file = File::open(full_path).ok()?;
        Some(Box::new(BufReader::new(file)) as Box<dyn FontData>)
    }

    #[inline]
    fn available<'p>(&'p self) -> &'p [FontInfo] {
        &self.infos
    }
}


/// The error type for font operations.
pub enum FontError {
    /// The font file is incorrect.
    InvalidFont(String),
    /// The font listing is incorrect.
    InvalidListing(String),
    /// A character requested for subsetting was not present in the source font.
    MissingCharacter(char),
    /// A requested or required table was not present.
    MissingTable(String),
    /// The table is unknown to the subsetting engine.
    UnsupportedTable(String),
    /// The font is not supported by the subsetting engine.
    UnsupportedFont(String),
    /// An I/O Error occured while reading the font program.
    Io(io::Error),
}

error_type! {
    err: FontError,
    res: FontResult,
    show: f => match err {
        FontError::InvalidFont(message) => write!(f, "invalid font: {}", message),
        FontError::InvalidListing(message) => write!(f, "invalid font listing: {}", message),
        FontError::MissingCharacter(c) => write!(f, "missing character: '{}'", c),
        FontError::MissingTable(table) => write!(f, "missing table: '{}'", table),
        FontError::UnsupportedTable(table) => write!(f, "unsupported table: {}", table),
        FontError::UnsupportedFont(message) => write!(f, "unsupported font: {}", message),
        FontError::Io(err) => write!(f, "io error: {}", err),
    },
    source: match err {
        FontError::Io(err) => Some(err),
        _ => None,
    },
    from: (io::Error, FontError::Io(err)),
    from: (OpentypeError, match err {
        OpentypeError::InvalidFont(message) => FontError::InvalidFont(message),
        OpentypeError::MissingTable(tag) => FontError::MissingTable(tag.to_string()),
        OpentypeError::Io(err) => FontError::Io(err),
    }),
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the font info macro.
    #[test]
    fn font_macro() {
        use FontClass::*;

        assert_eq!(font!["NotoSans", "Noto", Regular, SansSerif], FontInfo {
            classes: vec![
                Family("NotoSans".to_owned()), Family("Noto".to_owned()),
                Regular, SansSerif
            ]
        });

        assert_eq!(font!["NotoSerif", Serif, Italic, "Noto"], FontInfo {
            classes: vec![
                Family("NotoSerif".to_owned()), Serif, Italic,
                Family("Noto".to_owned())
            ],
        });
    }
}

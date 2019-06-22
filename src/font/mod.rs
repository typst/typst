//! Font loading and transforming.
//!
//! # Font handling
//! To do the typesetting, the typesetting engine needs font data. To be highly portable the engine
//! itself assumes nothing about the environment. To still work with fonts, the consumer of this
//! library has to add _font providers_ to their typesetting instance. These can be queried for font
//! data given flexible font filters specifying required font families and styles. A font provider
//! is a type implementing the [`FontProvider`](crate::font::FontProvider) trait.
//!
//! There is one [included font provider](crate::font::FileSystemFontProvider) that serves fonts
//! from a folder on the file system.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, BufReader};
use std::path::PathBuf;

use opentype::{Error as OpentypeError, OpenTypeReader};
use opentype::tables::{Header, Name, CharMap, HorizontalMetrics, Post, OS2};
use opentype::types::{MacStyleFlags, NameEntry};

pub use self::loader::{FontLoader, FontQuery};
use self::subset::Subsetter;
use crate::size::Size;

mod loader;
mod subset;


/// A loaded and parsed font program.
#[derive(Debug, Clone)]
pub struct Font {
    /// The base name of the font.
    pub name: String,
    /// The raw bytes of the font program.
    pub program: Vec<u8>,
    /// A mapping from character codes to glyph ids.
    pub mapping: HashMap<char, u16>,
    /// The widths of the glyphs indexed by glyph id.
    pub widths: Vec<Size>,
    /// The fallback glyph.
    pub default_glyph: u16,
    /// The typesetting-relevant metrics of this font.
    pub metrics: FontMetrics,
}

impl Font {
    /// Create a new font from a raw font program.
    pub fn new(program: Vec<u8>) -> FontResult<Font> {
        // Create an OpentypeReader to parse the font tables.
        let cursor = Cursor::new(&program);
        let mut reader = OpenTypeReader::new(cursor);

        // Read the relevant tables
        // (all of these are required by the OpenType specification, so we expect them).
        let head = reader.read_table::<Header>()?;
        let name = reader.read_table::<Name>()?;
        let os2 = reader.read_table::<OS2>()?;
        let cmap = reader.read_table::<CharMap>()?;
        let hmtx = reader.read_table::<HorizontalMetrics>()?;
        let post = reader.read_table::<Post>()?;

        // Create a conversion function between font units and sizes.
        let font_unit_ratio = 1.0 / (head.units_per_em as f32);
        let font_unit_to_size = |x| Size::pt(font_unit_ratio * x as f32);

        // Find out the name of the font.
        let font_name = name.get_decoded(NameEntry::PostScriptName)
            .unwrap_or_else(|| "unknown".to_owned());

        // Convert the widths from font units to sizes.
        let widths = hmtx.metrics.iter().map(|m| font_unit_to_size(m.advance_width)).collect();

        // Calculate the typesetting-relevant metrics.
        let metrics = FontMetrics {
            italic: head.mac_style.contains(MacStyleFlags::ITALIC),
            monospace: post.is_fixed_pitch,
            italic_angle: post.italic_angle.to_f32(),
            bounding_box: [
                font_unit_to_size(head.x_min),
                font_unit_to_size(head.y_min),
                font_unit_to_size(head.x_max),
                font_unit_to_size(head.y_max),
            ],
            ascender: font_unit_to_size(os2.s_typo_ascender),
            descender: font_unit_to_size(os2.s_typo_descender),
            cap_height: font_unit_to_size(os2.s_cap_height.unwrap_or(os2.s_typo_ascender)),
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

    /// Map a character to it's glyph index.
    #[inline]
    pub fn map(&self, c: char) -> u16 {
        self.mapping.get(&c).map(|&g| g).unwrap_or(self.default_glyph)
    }

    /// Encode the given text for this font (into glyph ids).
    #[inline]
    pub fn encode(&self, text: &str) -> Vec<u8> {
        // Each glyph id takes two bytes that we encode in big endian.
        let mut bytes = Vec::with_capacity(2 * text.len());
        for glyph in text.chars().map(|c| self.map(c)) {
            bytes.push((glyph >> 8) as u8);
            bytes.push((glyph & 0xff) as u8);
        }
        bytes
    }

    /// Generate a subsetted version of this font including only the chars listed in `chars`.
    ///
    /// All needed tables will be included (returning an error if a table was not present in the
    /// source font) and optional tables will be included if they were present in the source font.
    /// All other tables will be dropped.
    #[inline]
    pub fn subsetted<C, I, S>(&self, chars: C, needed_tables: I, optional_tables: I)
        -> Result<Font, FontError>
    where
        C: IntoIterator<Item=char>,
        I: IntoIterator<Item=S>,
        S: AsRef<str>
    {
        Subsetter::subset(self, chars, needed_tables, optional_tables)
    }
}

/// Font metrics relevant to the typesetting or exporting processes.
#[derive(Debug, Copy, Clone)]
pub struct FontMetrics {
    /// Whether the font is italic.
    pub italic: bool,
    /// Whether font is monospace.
    pub monospace: bool,
    /// The angle of text in italics.
    pub italic_angle: f32,
    /// The glyph bounding box: [x_min, y_min, x_max, y_max],
    pub bounding_box: [Size; 4],
    /// The typographics ascender.
    pub ascender: Size,
    /// The typographics descender.
    pub descender: Size,
    /// The approximate height of capital letters.
    pub cap_height: Size,
    /// The weight class of the font.
    pub weight_class: u16,
}

/// Categorizes a font.
///
/// Can be constructed conveniently with the [`font`] macro.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontInfo {
    /// The font families this font is part of.
    pub classes: Vec<FontClass>,
}

impl FontInfo {
    /// Create a new font info from an iterator of classes.
    pub fn new<I>(classes: I) -> FontInfo where I: IntoIterator<Item=FontClass> {
        FontInfo { classes: classes.into_iter().collect() }
    }
}

/// A class of fonts.
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
/// The font _Noto Sans_ in regular typeface.
/// ```
/// # use typeset::font;
/// font!["NotoSans", "Noto", Regular, SansSerif];
/// ```
///
/// The font _Noto Serif_ in italics and boldface.
/// ```
/// # use typeset::font;
/// font!["NotoSerif", "Noto", Bold, Italic, Serif];
/// ```
///
/// The font _Arial_ in italics.
/// ```
/// # use typeset::font;
/// font!["Arial", Italic, SansSerif];
/// ```
///
/// The font _Noto Emoji_, which works with all base families. 🙂
/// ```
/// # use typeset::font;
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

/// A type that provides fonts.
pub trait FontProvider {
    /// Returns a font with the given info if this provider has one.
    fn get(&self, info: &FontInfo) -> Option<Box<dyn FontData>>;

    /// The available fonts this provider can serve. While these should generally be retrievable
    /// through the `get` method, it does not have to be guaranteed that a font info, that is
    /// contained, here yields a `Some` value when passed into `get`.
    fn available<'p>(&'p self) -> &'p [FontInfo];
}

/// A wrapper trait around `Read + Seek`.
///
/// This type is needed because currently you can't make a trait object with two traits, like
/// `Box<dyn Read + Seek>`. Automatically implemented for all types that are [`Read`] and [`Seek`].
pub trait FontData: Read + Seek {}
impl<T> FontData for T where T: Read + Seek {}

/// A font provider serving fonts from a folder on the local file system.
#[derive(Debug)]
pub struct FileSystemFontProvider {
    /// The root folder.
    base: PathBuf,
    /// Paths of the fonts relative to the `base` path.
    paths: Vec<PathBuf>,
    /// The information for the font with the same index in `paths`.
    infos: Vec<FontInfo>,
}

impl FileSystemFontProvider {
    /// Create a new provider from a folder and an iterator of pairs of font paths and font infos.
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
    #[inline]
    pub fn new<B, I, P>(base: B, infos: I) -> FileSystemFontProvider
    where
        B: Into<PathBuf>,
        I: IntoIterator<Item = (P, FontInfo)>,
        P: Into<PathBuf>,
    {
        // Find out how long the iterator is at least, to reserve the correct capacity for the
        // vectors.
        let iter = infos.into_iter();
        let min = iter.size_hint().0;

        // Split the iterator into two seperated vectors.
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
}

impl FontProvider for FileSystemFontProvider {
    #[inline]
    fn get(&self, info: &FontInfo) -> Option<Box<dyn FontData>> {
        // Find the index of the font in both arrays (early exit if there is no match).
        let index = self.infos.iter().position(|i| i == info)?;

        // Open the file and return a boxed reader operating on it.
        let path = &self.paths[index];
        let file = File::open(self.base.join(path)).ok()?;
        Some(Box::new(BufReader::new(file)) as Box<FontData>)
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
    /// A requested table was not present in the source font.
    MissingTable(String),
    /// The table is unknown to the subsetting engine.
    UnsupportedTable(String),
    /// A character requested for subsetting was not present in the source font.
    MissingCharacter(char),
    /// An I/O Error occured while reading the font program.
    Io(io::Error),
}

error_type! {
    err: FontError,
    res: FontResult,
    show: f => match err {
        FontError::InvalidFont(message) => write!(f, "invalid font: {}", message),
        FontError::MissingTable(table) => write!(f, "missing table: {}", table),
        FontError::UnsupportedTable(table) => write!(f, "unsupported table: {}", table),
        FontError::MissingCharacter(c) => write!(f, "missing character: '{}'", c),
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
        _ => panic!("unexpected extensible variant"),
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

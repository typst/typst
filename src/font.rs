//! Reading of metrics and font data from _OpenType_ and _TrueType_ font files.

#![allow(unused_variables)]

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use byteorder::{BE, ReadBytesExt};


/// A loaded opentype (or truetype) font.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The PostScript name of this font.
    pub name: String,
}

impl Font {
    /// Create a new font from a byte source.
    pub fn new<R>(data: &mut R) -> FontResult<Font> where R: Read + Seek {
        OpenTypeReader::new(data).read()
    }
}

/// Built-in fonts.
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum BuiltinFont {
    Courier,
    CourierBold,
    CourierOblique,
    CourierBoldOblique,
    Helvetica,
    HelveticaBold,
    HelveticaOblique,
    HelveticaBoldOblique,
    TimesRoman,
    TimesBold,
    TimeItalic,
    TimeBoldItalic,
    Symbol,
    ZapfDingbats,
}

impl BuiltinFont {
    /// The name of the font.
    pub fn name(&self) -> &'static str {
        use BuiltinFont::*;
        match self {
            Courier => "Courier",
            CourierBold => "Courier-Bold",
            CourierOblique => "Courier-Oblique",
            CourierBoldOblique => "Courier-BoldOblique",
            Helvetica => "Helvetica",
            HelveticaBold => "Helvetica-Bold",
            HelveticaOblique => "Helvetica-Oblique",
            HelveticaBoldOblique => "Helvetica-BoldOblique",
            TimesRoman => "Times-Roman",
            TimesBold => "Times-Bold",
            TimeItalic => "Time-Italic",
            TimeBoldItalic => "Time-BoldItalic",
            Symbol => "Symbol",
            ZapfDingbats => "ZapfDingbats",
        }
    }
}


/// Result type used for tokenization.
type FontResult<T> = std::result::Result<T, LoadingError>;

/// A failure when loading a font.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LoadingError {
    /// A message describing the error.
    pub message: String,
}

impl From<io::Error> for LoadingError {
    fn from(err: io::Error) -> LoadingError {
        LoadingError { message: format!("io error: {}", err) }
    }
}

impl fmt::Display for LoadingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "font loading error: {}", self.message)
    }
}


/// Reads a font from a _OpenType_ or _TrueType_ font file.
struct OpenTypeReader<'r, R> where R: Read + Seek {
    data: &'r mut R,
    font: Font,
    table_records: Vec<TableRecord>,
}

/// Used to identify a table, design-variation axis, script,
/// language system, feature, or baseline.
#[derive(Clone, PartialEq)]
struct Tag(pub [u8; 4]);

impl PartialEq<&str> for Tag {
    fn eq(&self, other: &&str) -> bool {
        other.as_bytes() == &self.0
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let a = self.0;
        write!(f, "{}{}{}{}", a[0] as char, a[1] as char, a[2] as char, a[3] as char)
    }
}

/// Stores information about one table.
#[derive(Debug, Clone, PartialEq)]
struct TableRecord {
    table: Tag,
    check_sum: u32,
    offset: u32,
    length: u32,
}

impl<'r, R> OpenTypeReader<'r, R> where R: Read + Seek {
    /// Create a new reader from a byte source.
    pub fn new(data: &'r mut R) -> OpenTypeReader<'r, R> {
        OpenTypeReader {
            data,
            font: Font {
                name: String::new(),
            },
            table_records: vec![],
        }
    }

    /// Read the font from the byte source.
    pub fn read(mut self) -> FontResult<Font> {
        self.read_table_records()?;
        self.read_name_table()?;

        Ok(self.font)
    }

    /// Read the offset table.
    fn read_table_records(&mut self) -> FontResult<()> {
        let sfnt_version = self.data.read_u32::<BE>()?;
        let num_tables = self.data.read_u16::<BE>()?;
        let search_range = self.data.read_u16::<BE>()?;
        let entry_selector = self.data.read_u16::<BE>()?;
        let range_shift = self.data.read_u16::<BE>()?;

        let outlines = match sfnt_version {
            0x00010000 => "truetype",
            0x4F54544F => "cff",
            _ => return self.err("unsuported font outlines"),
        };

        for _ in 0 .. num_tables {
            let table = self.read_tag()?;
            let check_sum = self.data.read_u32::<BE>()?;
            let offset = self.data.read_u32::<BE>()?;
            let length = self.data.read_u32::<BE>()?;

            self.table_records.push(TableRecord {
                table,
                check_sum,
                offset,
                length,
            });
        }

        Ok(())
    }

    /// Read the name table (gives general information about the font).
    fn read_name_table(&mut self) -> FontResult<()> {
        let table = match self.table_records.iter().find(|record| record.table == "name") {
            Some(table) => table,
            None => return self.err("missing 'name' table"),
        };

        self.data.seek(SeekFrom::Start(table.offset as u64))?;

        let format = self.data.read_u16::<BE>()?;
        let count = self.data.read_u16::<BE>()?;
        let string_offset = self.data.read_u16::<BE>()?;

        let storage = (table.offset + string_offset as u32) as u64;

        let mut name = None;

        for _ in 0 .. count {
            let platform_id = self.data.read_u16::<BE>()?;
            let encoding_id = self.data.read_u16::<BE>()?;
            let language_id = self.data.read_u16::<BE>()?;
            let name_id = self.data.read_u16::<BE>()?;
            let length = self.data.read_u16::<BE>()?;
            let offset = self.data.read_u16::<BE>()?;

            // Postscript name is what we are interested in
            if name_id == 6 && platform_id == 3 && encoding_id == 1 {
                if length % 2 != 0 {
                    return self.err("invalid encoded name");
                }

                self.data.seek(SeekFrom::Start(storage + offset as u64))?;
                let mut buffer = Vec::with_capacity(length as usize / 2);

                for _ in 0 .. length / 2 {
                    buffer.push(self.data.read_u16::<BE>()?);
                }

                name = match String::from_utf16(&buffer) {
                    Ok(string) => Some(string),
                    Err(_) => return self.err("invalid encoded name"),
                };

                break;
            }
        }

        self.font.name = match name {
            Some(name) => name,
            None => return self.err("missing postscript font name"),
        };

        Ok(())
    }

    /// Read a tag (array of four u8's).
    fn read_tag(&mut self) -> FontResult<Tag> {
        let mut tag = [0u8; 4];
        self.data.read(&mut tag)?;
        Ok(Tag(tag))
    }

    /// Gives a font loading error with a message.
    fn err<T, S: Into<String>>(&self, message: S) -> FontResult<T> {
        Err(LoadingError { message: message.into() })
    }
}


#[cfg(test)]
mod font_tests {
    use super::*;

    /// Test if the loaded font is the same as the expected font.
    fn test(path: &str, font: Font) {
        let mut file = std::fs::File::open(path).unwrap();
        assert_eq!(Font::new(&mut file), Ok(font));
    }

    #[test]
    fn opentype() {
        test("../fonts/NotoSerif-Regular.ttf", Font {
            name: "NotoSerif".to_owned(),
        });
        test("../fonts/NotoSansMath-Regular.ttf", Font {
            name: "NotoSansMath-Regular".to_owned(),
        });
    }
}

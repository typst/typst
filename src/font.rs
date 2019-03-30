//! Font loading, utility and subsetting.

#![macro_use]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use byteorder::{BE, ReadBytesExt, WriteBytesExt};
use opentype::{Error as OpentypeError, OpenTypeReader, Outlines, TableRecord, Tag};
use opentype::tables::{Header, Name, CharMap, MaximumProfile, HorizontalMetrics, Post, OS2};
use opentype::global::{MacStyleFlags, NameEntry};
use crate::doc::{Size, FontFamily};


/// An font wrapper which allows to subset a font.
#[derive(Debug, Clone, PartialEq)]
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
    /// The relevant metrics of this font.
    pub metrics: FontMetrics,
}

/// Font metrics relevant to the typesetting engine.
#[derive(Debug, Clone, PartialEq)]
pub struct FontMetrics {
    /// Whether the font is italic.
    pub is_italic: bool,
    /// Whether font is fixed pitch.
    pub is_fixed_pitch: bool,
    /// The angle of italics.
    pub italic_angle: f32,
    /// The glyph bounding box: [x_min, y_min, x_max, y_max],
    pub bounding_box: [Size; 4],
    /// The typographics ascender relevant for line spacing.
    pub ascender: Size,
    /// The typographics descender relevant for line spacing.
    pub descender: Size,
    /// The approximate height of capital letters.
    pub cap_height: Size,
    /// The weight class of the font.
    pub weight_class: u16,
}

impl Font {
    /// Create a new font from a font program.
    pub fn new(program: Vec<u8>) -> FontResult<Font> {
        // Create opentype reader to parse font tables
        let mut readable = Cursor::new(&program);
        let mut reader = OpenTypeReader::new(&mut readable);

        // Read the relevant tables
        // (all of these are required by the OpenType specification)
        let head = reader.read_table::<Header>()?;
        let name = reader.read_table::<Name>()?;
        let os2 = reader.read_table::<OS2>()?;
        let cmap = reader.read_table::<CharMap>()?;
        let hmtx = reader.read_table::<HorizontalMetrics>()?;
        let post = reader.read_table::<Post>()?;

        // Create conversion function between font units and sizes
        let font_unit_ratio = 1.0 / (head.units_per_em as f32);
        let font_unit_to_size = |x| Size::from_points(font_unit_ratio * x as f32);

        // Find out the name of the font
        let font_name = name.get_decoded(NameEntry::PostScriptName)
            .unwrap_or_else(|| "unknown".to_owned());

        // Convert the widths
        let widths = hmtx.metrics.iter().map(|m| font_unit_to_size(m.advance_width)).collect();

        // Calculate some metrics
        let metrics = FontMetrics {
            is_italic: head.mac_style.contains(MacStyleFlags::ITALIC),
            is_fixed_pitch: post.is_fixed_pitch,
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
        let mut bytes = Vec::with_capacity(2 * text.len());
        for glyph in text.chars().map(|c| self.map(c)) {
            bytes.push((glyph >> 8) as u8);
            bytes.push((glyph & 0xff) as u8);
        }
        bytes
    }

    /// Generate a subsetted version of this font including only the chars listed in
    /// `chars`.
    ///
    /// All needed tables will be included (returning an error if a table was not present
    /// in the  source font) and optional tables will be included if there were present
    /// in the source font. All other tables will be dropped.
    pub fn subsetted<C, I1, S1, I2, S2>(
        &self,
        chars: C,
        needed_tables: I1,
        optional_tables: I2
    ) -> Result<Font, FontError>
    where
        C: IntoIterator<Item=char>,
        I1: IntoIterator<Item=S1>, S1: AsRef<str>,
        I2: IntoIterator<Item=S2>, S2: AsRef<str>
    {
        let mut chars: Vec<char> = chars.into_iter().collect();
        chars.sort();

        let mut reader = OpenTypeReader::from_slice(&self.program);
        let outlines = reader.outlines()?;
        let tables = reader.tables()?.to_vec();

        Subsetter {
            font: &self,
            reader,
            outlines,
            tables,
            cmap: None,
            hmtx: None,
            loca: None,
            glyphs: Vec::with_capacity(chars.len()),
            chars,
            records: Vec::new(),
            body: Vec::new(),
        }.subset(needed_tables, optional_tables)
    }
}

#[derive(Debug)]
struct Subsetter<'p> {
    // Original font
    font: &'p Font,
    reader: OpenTypeReader<Cursor<&'p [u8]>>,
    outlines: Outlines,
    tables: Vec<TableRecord>,
    cmap: Option<CharMap>,
    hmtx: Option<HorizontalMetrics>,
    loca: Option<Vec<u32>>,
    glyphs: Vec<u16>,

    // Subsetted font
    chars: Vec<char>,
    records: Vec<TableRecord>,
    body: Vec<u8>,
}

impl<'p> Subsetter<'p> {
    fn subset<I1, S1, I2, S2>(mut self, needed_tables: I1, optional_tables: I2)
    -> FontResult<Font>
    where
        I1: IntoIterator<Item=S1>, S1: AsRef<str>,
        I2: IntoIterator<Item=S2>, S2: AsRef<str>
    {
        // Find out which glyphs to include based on which characters we want
        // and which glyphs are used by composition.
        self.build_glyphs()?;

        // Iterate through the needed tables first
        for table in needed_tables.into_iter() {
            let table = table.as_ref();
            let tag: Tag = table.parse()
                .map_err(|_| FontError::UnsupportedTable(table.to_string()))?;

            if self.contains(tag) {
                self.write_table(tag)?;
            } else {
                return Err(FontError::MissingTable(tag.to_string()));
            }
        }

        // Now iterate through the optional tables
        for table in optional_tables.into_iter() {
            let table = table.as_ref();
            let tag: Tag = table.parse()
                .map_err(|_| FontError::UnsupportedTable(table.to_string()))?;

            if self.contains(tag) {
                self.write_table(tag)?;
            }
        }

        self.write_header()?;

        let widths = self.glyphs.iter()
            .map(|&glyph| self.font.widths.get(glyph as usize).map(|&w| w)
                .take_invalid("missing glyph metrics"))
            .collect::<FontResult<Vec<_>>>()?;

        let mapping = self.chars.into_iter().enumerate().map(|(i, c)| (c, i as u16))
            .collect::<HashMap<char, u16>>();

        Ok(Font {
            name: self.font.name.clone(),
            program: self.body,
            mapping,
            widths,
            default_glyph: self.font.default_glyph,
            metrics: self.font.metrics.clone(),
        })
    }

    fn build_glyphs(&mut self) -> FontResult<()> {
        self.read_cmap()?;
        let cmap = self.cmap.as_ref().unwrap();

        for &c in &self.chars {
            self.glyphs.push(cmap.get(c).ok_or_else(|| FontError::MissingCharacter(c))?)
        }

        self.glyphs.push(self.font.default_glyph);

        // Composite glyphs may need additional glyphs we have not yet in our list.
        // So now we have a look at the glyf table to check that and add glyphs
        // we need additionally.
        if self.contains("glyf".parse().unwrap()) {
            self.read_loca()?;
            let loca = self.loca.as_ref().unwrap();
            let table = self.get_table_data("glyf".parse().unwrap())?;

            let mut i = 0;
            while i < self.glyphs.len() {
                let glyph = self.glyphs[i];

                let start = *loca.get(glyph as usize).take_bytes()? as usize;
                let end = *loca.get(glyph as usize + 1).take_bytes()? as usize;

                let glyph = table.get(start..end).take_bytes()?;

                if end > start {
                    let mut cursor = Cursor::new(&glyph);
                    let num_contours = cursor.read_i16::<BE>()?;

                    // This is a composite glyph
                    if num_contours < 0 {
                        cursor.seek(SeekFrom::Current(8))?;
                        loop {
                            let flags = cursor.read_u16::<BE>()?;
                            let glyph_index = cursor.read_u16::<BE>()?;

                            if self.glyphs.iter().rev().find(|&&x| x == glyph_index).is_none() {
                                self.glyphs.push(glyph_index);
                            }

                            // This was the last component
                            if flags & 0x0020 == 0 {
                                break;
                            }

                            let args_len = if flags & 0x0001 == 1 { 4 } else { 2 };
                            cursor.seek(SeekFrom::Current(args_len))?;
                        }
                    }
                }

                i += 1;
            }
        }

        Ok(())
    }

    fn write_header(&mut self) -> FontResult<()> {
        // Create an output buffer
        let header_len = 12 + self.records.len() * 16;
        let mut header = Vec::with_capacity(header_len);

        let num_tables = self.records.len() as u16;

        // The highester power lower than the table count.
        let mut max_power = 1u16;
        while max_power * 2 <= num_tables {
            max_power *= 2;
        }
        max_power = std::cmp::min(max_power, num_tables);

        let search_range = max_power * 16;
        let entry_selector = (max_power as f32).log2() as u16;
        let range_shift = num_tables * 16 - search_range;

        // Write the base header
        header.write_u32::<BE>(match self.outlines {
            Outlines::TrueType => 0x00010000,
            Outlines::CFF => 0x4f54544f,
        })?;
        header.write_u16::<BE>(num_tables)?;
        header.write_u16::<BE>(search_range)?;
        header.write_u16::<BE>(entry_selector)?;
        header.write_u16::<BE>(range_shift)?;

        // Write the table records
        for record in &self.records {
            header.extend(record.tag.value());
            header.write_u32::<BE>(record.check_sum)?;
            header.write_u32::<BE>(header_len as u32 + record.offset)?;
            header.write_u32::<BE>(record.length)?;
        }

        header.append(&mut self.body);
        self.body = header;

        Ok(())
    }

    fn write_table(&mut self, tag: Tag) -> FontResult<()> {
        match tag.value() {
            b"head" | b"cvt " | b"prep" | b"fpgm" | b"name" | b"post" | b"OS/2" => {
                self.copy_table(tag)
            },
            b"hhea" => {
                let table = self.get_table_data(tag)?;
                let glyph_count = self.glyphs.len() as u16;
                self.write_table_body(tag, |this| {
                    this.body.extend(&table[..table.len() - 2]);
                    Ok(this.body.write_u16::<BE>(glyph_count)?)
                })
            },
            b"maxp" => {
                let table = self.get_table_data(tag)?;
                let glyph_count = self.glyphs.len() as u16;
                self.write_table_body(tag, |this| {
                    this.body.extend(&table[..4]);
                    this.body.write_u16::<BE>(glyph_count)?;
                    Ok(this.body.extend(&table[6..]))
                })
            },
            b"hmtx" => {
                self.write_table_body(tag, |this| {
                    this.read_hmtx()?;
                    let metrics = this.hmtx.as_ref().unwrap();

                    for &glyph in &this.glyphs {
                        let metrics = metrics.get(glyph).take_invalid("missing glyph metrics")?;

                        this.body.write_i16::<BE>(metrics.advance_width)?;
                        this.body.write_i16::<BE>(metrics.left_side_bearing)?;
                    }
                    Ok(())
                })
            },
            b"loca" => {
                self.write_table_body(tag, |this| {
                    this.read_loca()?;
                    let loca = this.loca.as_ref().unwrap();

                    let mut offset = 0;
                    for &glyph in &this.glyphs {
                        this.body.write_u32::<BE>(offset)?;
                        let len = loca.get(glyph as usize + 1).take_bytes()?
                                - loca.get(glyph as usize).take_bytes()?;
                        offset += len;
                    }
                    this.body.write_u32::<BE>(offset)?;
                    Ok(())
                })
            },

            b"glyf" => {
                self.write_table_body(tag, |this| {
                    this.read_loca()?;
                    let loca = this.loca.as_ref().unwrap();
                    let table = this.get_table_data(tag)?;

                    for &glyph in &this.glyphs {
                        let start = *loca.get(glyph as usize).take_bytes()? as usize;
                        let end = *loca.get(glyph as usize + 1).take_bytes()? as usize;

                        let mut data = table.get(start..end).take_bytes()?.to_vec();

                        if end > start {
                            let mut cursor = Cursor::new(&mut data);
                            let num_contours = cursor.read_i16::<BE>()?;

                            // This is a composite glyph
                            if num_contours < 0 {
                                cursor.seek(SeekFrom::Current(8))?;
                                loop {
                                    let flags = cursor.read_u16::<BE>()?;

                                    let glyph_index = cursor.read_u16::<BE>()?;
                                    let new_glyph_index = this.glyphs.iter()
                                        .position(|&g| g == glyph_index)
                                        .take_invalid("referenced non-existent glyph")? as u16;

                                    cursor.seek(SeekFrom::Current(-2))?;
                                    cursor.write_u16::<BE>(new_glyph_index)?;

                                    // This was the last component
                                    if flags & 0x0020 == 0 {
                                        break;
                                    }


                                    let args_len = if flags & 0x0001 == 1 { 4 } else { 2 };
                                    cursor.seek(SeekFrom::Current(args_len))?;
                                }
                            }
                        }

                        this.body.extend(data);
                    }
                    Ok(())
                })
            },

            b"cmap" => {
                // Always uses format 12 for simplicity
                self.write_table_body(tag, |this| {
                    // Find out which chars are in consecutive groups
                    let mut groups = Vec::new();
                    let len = this.chars.len();
                    let mut i = 0;
                    while i < len {
                        let start = i;
                        while i + 1 < len && this.chars[i+1] as u32 == this.chars[i] as u32 + 1 {
                            i += 1;
                        }
                        groups.push((this.chars[start], this.chars[i], start));
                        i += 1;
                    }

                    // Table header
                    this.body.write_u16::<BE>(0)?;
                    this.body.write_u16::<BE>(1)?;
                    this.body.write_u16::<BE>(3)?;
                    this.body.write_u16::<BE>(1)?;
                    this.body.write_u32::<BE>(12)?;

                    // Subtable header
                    this.body.write_u16::<BE>(12)?;
                    this.body.write_u16::<BE>(0)?;
                    this.body.write_u32::<BE>((16 + 12 * groups.len()) as u32)?;
                    this.body.write_u32::<BE>(0)?;
                    this.body.write_u32::<BE>(groups.len() as u32)?;

                    // Subtable body
                    for group in &groups {
                        this.body.write_u32::<BE>(group.0 as u32)?;
                        this.body.write_u32::<BE>(group.1 as u32)?;
                        this.body.write_u32::<BE>(group.2 as u32)?;
                    }

                    Ok(())
                })
            },

            _ => Err(FontError::UnsupportedTable(tag.to_string())),
        }
    }

    fn copy_table(&mut self, tag: Tag) -> FontResult<()> {
        self.write_table_body(tag, |this| {
            let table = this.get_table_data(tag)?;
            Ok(this.body.extend(table))
        })
    }

    fn write_table_body<F>(&mut self, tag: Tag, writer: F) -> FontResult<()>
    where F: FnOnce(&mut Self) -> FontResult<()> {
        let start = self.body.len();
        writer(self)?;
        let end = self.body.len();
        while (self.body.len() - start) % 4 != 0 {
            self.body.push(0);
        }

        Ok(self.records.push(TableRecord {
            tag,
            check_sum: calculate_check_sum(&self.body[start..]),
            offset: start as u32,
            length: (end - start) as u32,
        }))
    }

    fn get_table_data(&self, tag: Tag) -> FontResult<&'p [u8]> {
        let record = match self.tables.binary_search_by_key(&tag, |r| r.tag) {
            Ok(index) => &self.tables[index],
            Err(_) => return Err(FontError::MissingTable(tag.to_string())),
        };

        self.font.program
            .get(record.offset as usize .. (record.offset + record.length) as usize)
            .take_bytes()
    }

    fn contains(&self, tag: Tag) -> bool {
        self.tables.binary_search_by_key(&tag, |r| r.tag).is_ok()
    }

    fn read_cmap(&mut self) -> FontResult<()> {
        Ok(if self.cmap.is_none() {
            self.cmap = Some(self.reader.read_table::<CharMap>()?);
        })
    }

    fn read_hmtx(&mut self) -> FontResult<()> {
        Ok(if self.hmtx.is_none() {
            self.hmtx = Some(self.reader.read_table::<HorizontalMetrics>()?);
        })
    }

    fn read_loca(&mut self) -> FontResult<()> {
        Ok(if self.loca.is_none() {
            let mut table = self.get_table_data("loca".parse().unwrap())?;
            let format = self.reader.read_table::<Header>()?.index_to_loc_format;
            let count = self.reader.read_table::<MaximumProfile>()?.num_glyphs + 1;

            let loca = if format == 0 {
                (0..count).map(|_| table.read_u16::<BE>()
                    .map(|x| (x as u32) * 2))
                    .collect::<io::Result<Vec<u32>>>()
            } else {
                (0..count).map(|_| table.read_u32::<BE>())
                    .collect::<io::Result<Vec<u32>>>()
            }?;

            self.loca = Some(loca);
        })
    }
}

/// Calculate a checksum over the sliced data as sum of u32's.
/// The data length has to be a multiple of four.
fn calculate_check_sum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    data.chunks_exact(4).for_each(|c| {
        sum = sum.wrapping_add(
            ((c[0] as u32) << 24)
          + ((c[1] as u32) << 16)
          + ((c[2] as u32) << 8)
          + (c[3] as u32)
        );
    });
    sum
}

trait TakeInvalid<T>: Sized {
    /// Pull the type out of the option, returning a subsetting error
    /// about an invalid font wrong.
    fn take_invalid<S: Into<String>>(self, message: S) -> FontResult<T>;

    /// Pull the type out of the option, returning an error about missing
    /// bytes if it is nothing.
    fn take_bytes(self) -> FontResult<T> {
        self.take_invalid("expected more bytes")
    }
}

impl<T> TakeInvalid<T> for Option<T> {
    fn take_invalid<S: Into<String>>(self, message: S) -> FontResult<T> {
        self.ok_or(FontError::InvalidFont(message.into()))
    }
}

///////////////////////////////////////////////////////////////////////////////

/// A type that provides fonts matching given criteria.
pub trait FontProvider {
    /// Returns a font matching the configuration
    /// if this provider has a matching font.
    fn provide(&self, config: &FontConfig) -> Option<Box<dyn FontSource>>;
}

/// A wrapper trait around `Read + Seek` to allow for making a trait object.
///
/// Automatically implemented for all types that are [`Read`] and [`Seek`].
pub trait FontSource: Read + Seek {}
impl<T> FontSource for T where T: Read + Seek {}

/// Criteria to filter fonts.
#[derive(Debug, Clone, PartialEq)]
pub struct FontConfig {
    /// The font families we are looking for.
    pub families: Vec<FontFamily>,
    /// If some, matches only italic/non-italic fonts, otherwise any.
    pub italic: Option<bool>,
    /// If some, matches only bold/non-bold fonts, otherwise any.
    pub bold: Option<bool>,
}

impl FontConfig {
    /// Create a new font config with the given families.
    ///
    /// All other fields are set to [`None`] and match anything.
    pub fn new(families: Vec<FontFamily>) -> FontConfig {
        FontConfig {
            families,
            italic: None,
            bold: None,
        }
    }

    /// Set the italic value to something.
    pub fn italic(&mut self, italic: bool) -> &mut Self {
        self.italic = Some(italic);
        self
    }

    /// Set the bold value to something.
    pub fn bold(&mut self, bold: bool) -> &mut Self {
        self.bold = Some(bold);
        self
    }
}

/// A font provider that works on font files on the local file system.
pub struct FileFontProvider<'a> {
    root: PathBuf,
    specs: Vec<FileFontDescriptor<'a>>,
}

impl<'a> FileFontProvider<'a> {
    /// Create a new file font provider. The folder relative to which the `specs`
    /// contains the file paths, is given as `root`.
    pub fn new<P: 'a, I>(root: P, specs: I) -> FileFontProvider<'a>
    where
        I: IntoIterator<Item=FileFontDescriptor<'a>>,
        P: Into<PathBuf>
    {
        FileFontProvider {
            root: root.into(),
            specs: specs.into_iter().collect()
        }
    }
}

/// A type describing a font on the file system.
///
/// Can be constructed conventiently with the [`file_font`] macro.
pub struct FileFontDescriptor<'a> {
    /// The path to the font relative to the root.
    pub path: &'a Path,
    /// The font families this font is part of.
    pub families: Vec<FontFamily>,
    /// Whether the font is in italics.
    pub italic: bool,
    /// Whether the font is bold.
    pub bold: bool,
}

impl FileFontDescriptor<'_> {
    fn matches(&self, config: &FontConfig) -> bool {
        config.italic.map(|i| i == self.italic).unwrap_or(true)
        && config.bold.map(|i| i == self.bold).unwrap_or(true)
        && config.families.iter().any(|family| self.families.contains(family))
    }
}

/// Helper macro to create [file font descriptors](crate::font::FileFontDescriptor).
///
/// ```
/// # use typeset::file_font;
/// file_font!(
///     "NotoSans",              // Font family name
///     [SansSerif],             // Generic families
///     "NotoSans-Regular.ttf",  // Font file
///     false, false             // Bold & Italic
/// );
/// ```
#[macro_export]
macro_rules! file_font {
    ($family:expr, [$($generic:ident),*], $path:expr, $bold:expr, $italic:expr) => {{
        let mut families = vec![$crate::doc::FontFamily::Named($family.to_string())];
        families.extend([$($crate::doc::FontFamily::$generic),*].iter().cloned());
        $crate::font::FileFontDescriptor {
            path: std::path::Path::new($path),
            families,
            italic: $italic, bold: $bold,
        }
    }};
}

impl FontProvider for FileFontProvider<'_> {
    fn provide(&self, config: &FontConfig) -> Option<Box<dyn FontSource>> {
        self.specs.iter().find(|spec| spec.matches(&config)).map(|spec| {
            let file = std::fs::File::open(self.root.join(spec.path)).unwrap();
            Box::new(file) as Box<FontSource>
        })
    }
}

/// The error type for font operations.
pub enum FontError {
    /// The font file is incorrect.
    InvalidFont(String),
    /// A requested table was not present in the source font.
    MissingTable(String),
    /// The table is unknown to the subsetting engine (unimplemented or invalid).
    UnsupportedTable(String),
    /// A requested character was not present in the source font.
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

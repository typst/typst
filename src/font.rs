//! Font loading and transforming.
//!
//! # Font handling
//! To do the typesetting, the compiler needs font data. To be highly portable the compiler assumes
//! nothing about the environment. To still work with fonts, the consumer of this library has to
//! add _font providers_ to their compiler instance. These can be queried for font data given
//! flexible font filters specifying required font families and styles. A font provider is a type
//! implementing the [`FontProvider`](crate::font::FontProvider) trait.
//!
//! There is one [included font provider](crate::font::FileSystemFontProvider) that serves
//! fonts from a folder on the file system.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom, BufReader};
use std::path::PathBuf;

use byteorder::{BE, ReadBytesExt, WriteBytesExt};
use opentype::{Error as OpentypeError, OpenTypeReader, Outlines, TableRecord, Tag};
use opentype::tables::{Header, Name, CharMap, MaximumProfile, HorizontalMetrics, Post, OS2};
use opentype::global::{MacStyleFlags, NameEntry};

use crate::engine::Size;


/// A loaded font, containing relevant information for typesetting.
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
    /// The relevant metrics of this font.
    pub metrics: FontMetrics,
}

impl Font {
    /// Create a new font from a font program.
    pub fn new(program: Vec<u8>) -> FontResult<Font> {
        // Create opentype reader to parse font tables
        let cursor = Cursor::new(&program);
        let mut reader = OpenTypeReader::new(cursor);

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
    pub fn subsetted<C, I, S>(
        &self,
        chars: C,
        needed_tables: I,
        optional_tables: I,
    ) -> Result<Font, FontError>
    where
        C: IntoIterator<Item=char>,
        I: IntoIterator<Item=S>,
        S: AsRef<str>,
    {
        let mut chars: Vec<char> = chars.into_iter().collect();
        chars.sort();

        let mut reader = OpenTypeReader::from_slice(&self.program);
        let outlines = reader.outlines()?;
        let tables = reader.tables()?.to_vec();

        let subsetter = Subsetter {
            font: &self,
            reader,
            outlines,
            tables,
            cmap: None,
            hmtx: None,
            loca: None,
            glyphs: Vec::with_capacity(1 + chars.len()),
            chars,
            records: vec![],
            body: vec![],
        };

        subsetter.subset(needed_tables, optional_tables)
    }
}

/// Font metrics relevant to the typesetting engine.
#[derive(Debug, Copy, Clone)]
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

/// A type that provides fonts.
pub trait FontProvider {
    /// Returns the font with the given info if this provider has it.
    fn get(&self, info: &FontInfo) -> Option<Box<dyn FontData>>;

    /// The available fonts this provider can serve. While these should generally be retrievable
    /// through the `get` method, it is not guaranteed that a font info that is contained here
    /// yields a `Some` value when passed into `get`.
    fn available<'a>(&'a self) -> &'a [FontInfo];
}

/// A wrapper trait around `Read + Seek`.
///
/// This type is needed because currently you can't make a trait object
/// with two traits, like `Box<dyn Read + Seek>`.
/// Automatically implemented for all types that are [`Read`] and [`Seek`].
pub trait FontData: Read + Seek {}
impl<T> FontData for T where T: Read + Seek {}

/// Describes a font.
///
/// Can be constructed conveniently with the [`font_info`] macro.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FontInfo {
    /// The font families this font is part of.
    pub families: Vec<FontFamily>,
    /// Whether the font is in italics.
    pub italic: bool,
    /// Whether the font is bold.
    pub bold: bool,
}

/// A macro to create [FontInfos](crate::font::FontInfo) easily.
///
/// Accepts first a bracketed (ordered) list of font families. Allowed are string expressions
/// aswell as the three base families `SansSerif`, `Serif` and `Monospace`.
///
/// Then there may follow (separated by commas) the keywords `italic` and/or `bold`.
///
/// # Examples
/// The font _Noto Sans_ in regular typeface.
/// ```
/// # use typeset::font_info;
/// font_info!(["NotoSans", "Noto", SansSerif]);
/// ```
///
/// The font _Noto Serif_ in italics and boldface.
/// ```
/// # use typeset::font_info;
/// font_info!(["NotoSerif", "Noto", Serif], italic, bold);
/// ```
///
/// The font _Arial_ in italics.
/// ```
/// # use typeset::font_info;
/// font_info!(["Arial", SansSerif], italic);
/// ```
///
/// The font _Noto Emoji_, which works with all base families. 🙂
/// ```
/// # use typeset::font_info;
/// font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace]);
/// ```
#[macro_export]
macro_rules! font_info {
    // Entry point
    ([$($tts:tt)*] $(,$style:tt)*) => {{
        let mut families = Vec::new();
        font_info!(@__fam families, $($tts)*);

        #[allow(unused)] let mut italic = false;
        #[allow(unused)] let mut bold = false;
        $( font_info!(@__sty (italic, bold) $style); )*

        $crate::font::FontInfo { families, italic, bold }
    }};

    // Parse family list
    (@__fam $v:expr) => {};
    (@__fam $v:expr, $f:ident) => { $v.push(font_info!(@__gen $f)); };
    (@__fam $v:expr, $f:ident, $($tts:tt)*) => {
        font_info!(@__fam $v, $f);
        font_info!(@__fam $v, $($tts)*)
    };
    (@__fam $v:expr, $f:expr) => {
        $v.push( $crate::font::FontFamily::Named($f.to_string()));
    };
    (@__fam $v:expr, $f:expr, $($tts:tt)*) => {
        font_info!(@__fam $v, $f);
        font_info!(@__fam $v, $($tts)*)
    };

    // Parse styles (italic/bold)
    (@__sty ($i:ident, $b:ident) italic) => { $i = true; };
    (@__sty ($i:ident, $b:ident) bold) => { $b = true; };

    // Parse enum variants
    (@__gen SansSerif) => {  $crate::font::FontFamily::SansSerif };
    (@__gen Serif) => {  $crate::font::FontFamily::Serif };
    (@__gen Monospace) => {  $crate::font::FontFamily::Monospace };
}

/// A family of fonts (either generic or named).
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    Named(String),
}

/// A font provider serving fonts from a folder on the local file system.
pub struct FileSystemFontProvider {
    base: PathBuf,
    paths: Vec<PathBuf>,
    infos: Vec<FontInfo>,
}

impl FileSystemFontProvider {
    /// Create a new provider from a folder and an iterator of pairs of
    /// font paths and font infos.
    ///
    /// # Example
    /// Serve the two fonts `NotoSans-Regular` and `NotoSans-Italic` from the local
    /// folder `../fonts`.
    /// ```
    /// # use typeset::{font::FileSystemFontProvider, font_info};
    /// FileSystemFontProvider::new("../fonts", vec![
    ///     ("NotoSans-Regular.ttf", font_info!(["NotoSans", SansSerif])),
    ///     ("NotoSans-Italic.ttf", font_info!(["NotoSans", SansSerif], italic)),
    /// ]);
    /// ```
    #[inline]
    pub fn new<B, I, P>(base: B, infos: I) -> FileSystemFontProvider
    where
        B: Into<PathBuf>,
        I: IntoIterator<Item = (P, FontInfo)>,
        P: Into<PathBuf>,
    {
        // Find out how long the iterator is at least, to reserve the correct
        // capacity for the vectors.
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
        let index = self.infos.iter().position(|i| i == info)?;
        let path = &self.paths[index];
        let file = File::open(self.base.join(path)).ok()?;
        Some(Box::new(BufReader::new(file)) as Box<FontData>)
    }

    #[inline]
    fn available<'a>(&'a self) -> &'a [FontInfo] {
        &self.infos
    }
}

struct Subsetter<'d> {
    // Original font
    font: &'d Font,
    reader: OpenTypeReader<Cursor<&'d [u8]>>,
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

impl<'d> Subsetter<'d> {
    fn subset<I, S>(mut self, needed_tables: I, optional_tables: I) -> FontResult<Font>
    where I: IntoIterator<Item=S>, S: AsRef<str> {
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

        // Build the new widths.
        let widths = self.glyphs.iter()
            .map(|&glyph| {
                self.font.widths.get(glyph as usize).map(|&w| w)
                    .take_invalid("missing glyph metrics")
            }).collect::<FontResult<Vec<_>>>()?;

        // We add one to the index here because we added the default glyph to the front.
        let mapping = self.chars.into_iter().enumerate().map(|(i, c)| (c, 1 + i as u16))
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

        // The default glyph should be always present.
        self.glyphs.push(self.font.default_glyph);
        for &c in &self.chars {
            self.glyphs.push(cmap.get(c).ok_or_else(|| FontError::MissingCharacter(c))?)
        }

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

                        // Add one to the start because we inserted the default glyph in front.
                        let glyph = 1 + start;
                        groups.push((this.chars[start], this.chars[i], glyph));
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

    fn get_table_data(&self, tag: Tag) -> FontResult<&'d [u8]> {
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


#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the font info macro.
    #[test]
    fn font_info_macro() {
        use FontFamily::{SansSerif as S, Serif as F, Monospace as M};
        #[allow(non_snake_case)]
        fn N(family: &str) -> FontFamily { FontFamily::Named(family.to_string()) }

        assert_eq!(font_info!(["NotoSans", "Noto", SansSerif]), FontInfo {
            families: vec![N("NotoSans"), N("Noto"), S],
            italic: false,
            bold: false,
        });

        assert_eq!(font_info!(["NotoSerif", Serif, "Noto"], italic), FontInfo {
            families: vec![N("NotoSerif"), F, N("Noto")],
            italic: true,
            bold: false,
        });

        assert_eq!(font_info!(["NotoSans", "Noto", SansSerif], italic, bold), FontInfo {
            families: vec![N("NotoSans"), N("Noto"), S],
            italic: true,
            bold: true,
        });

        assert_eq!(font_info!(["NotoEmoji", "Noto", SansSerif, Serif, Monospace]), FontInfo {
            families: vec![N("NotoEmoji"), N("Noto"), S, F, M],
            italic: false,
            bold: false,
        });
    }
}

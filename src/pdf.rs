//! Writing of documents in the _PDF_ format.

use std::fmt;
use std::io::{self, Write, Cursor};
use std::collections::{HashMap, HashSet};
use pdf::{PdfWriter, Id, Rect, Version, Trailer};
use pdf::doc::{Catalog, PageTree, Page, Resource, Content};
use pdf::text::Text;
use pdf::font::{
    Type0Font, CMapEncoding, CIDFont, CIDFontType, CIDSystemInfo,
    WidthRecord, FontDescriptor, FontFlags, EmbeddedFont, GlyphUnit
};
use opentype::{OpenTypeReader, tables::{self, NameEntry, MacStyleFlags}};
use crate::doc::Document;
use crate::font::Font;


/// A type that is a sink for documents that can be written in the _PDF_ format.
pub trait WritePdf {
    /// Write a document into self, returning how many bytes were written.
    fn write_pdf(&mut self, doc: &Document) -> PdfResult<usize>;
}

impl<W: Write> WritePdf for W {
    fn write_pdf(&mut self, doc: &Document) -> PdfResult<usize> {
        PdfCreator::new(self, doc)?.write()
    }
}

/// Result type used for parsing.
type PdfResult<T> = std::result::Result<T, PdfWritingError>;

/// A failure while writing a _PDF_.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PdfWritingError {
    /// A message describing the error.
    message: String,
}

impl From<io::Error> for PdfWritingError {
    fn from(err: io::Error) -> PdfWritingError {
        PdfWritingError { message: format!("io error: {}", err) }
    }
}

impl From<opentype::Error> for PdfWritingError {
    fn from(err: opentype::Error) -> PdfWritingError {
        PdfWritingError { message: format!("{}", err) }
    }
}

impl From<crate::font::SubsettingError> for PdfWritingError {
    fn from(err: crate::font::SubsettingError) -> PdfWritingError {
        PdfWritingError { message: format!("{}", err) }
    }
}

impl fmt::Display for PdfWritingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pdf writing error: {}", self.message)
    }
}


/// Keeps track of the document while letting the pdf writer
/// generate the _PDF_.
struct PdfCreator<'a, W: Write> {
    writer: PdfWriter<'a, W>,
    doc: &'a Document,
    offsets: Offsets,
    font: PdfFont,
}

/// Offsets for the various groups of ids.
struct Offsets {
    catalog: Id,
    page_tree: Id,
    pages: (Id, Id),
    contents: (Id, Id),
    fonts: (Id, Id),
}

impl<'a, W: Write> PdfCreator<'a, W> {
    /// Create a new _PDF_ Creator.
    pub fn new(target: &'a mut W, doc: &'a Document) -> PdfResult<PdfCreator<'a, W>> {
        // Calculate a unique id for all object to come
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages = (page_tree + 1, page_tree + doc.pages.len() as Id);
        let content_count = doc.pages.iter().flat_map(|p| p.contents.iter()).count() as Id;
        let contents = (pages.1 + 1, pages.1 + content_count);
        let fonts = (contents.1 + 1, contents.1 + 4);

        // Find out which chars are used in this document.
        let mut chars = HashSet::new();
        for page in &doc.pages {
            for content in &page.contents {
                chars.extend(content.0.chars());
            }
        }

        // Create a subsetted pdf font.
        let data = std::fs::read(format!("../fonts/{}.ttf", doc.font))?;
        let font = PdfFont::new(&doc.font, data, chars)?;

        Ok(PdfCreator {
            writer: PdfWriter::new(target),
            doc,
            offsets: Offsets {
                catalog,
                page_tree,
                pages,
                contents,
                fonts,
            },
            font,
        })
    }

    /// Write the complete document.
    fn write(&mut self) -> PdfResult<usize> {
        // Header
        self.writer.write_header(&Version::new(1, 7))?;

        // Document catalog, page tree and pages
        self.write_pages()?;

        // Contents
        self.write_contents()?;

        // Fonts
        self.write_fonts()?;

        // Cross-reference table
        self.writer.write_xref_table()?;

        // Trailer
        self.writer.write_trailer(&Trailer::new(self.offsets.catalog))?;

        Ok(self.writer.written())
    }

    /// Write the document catalog, page tree and pages.
    fn write_pages(&mut self) -> PdfResult<()> {
        // The document catalog
        self.writer.write_obj(self.offsets.catalog, &Catalog::new(self.offsets.page_tree))?;

        // Root page tree
        self.writer.write_obj(self.offsets.page_tree, PageTree::new()
            .kids(self.offsets.pages.0 ..= self.offsets.pages.1)
            .resource(Resource::Font { nr: 1, id: self.offsets.fonts.0 })
        )?;

        // The page objects
        let mut id = self.offsets.pages.0;
        for page in &self.doc.pages {
            let width = page.size[0].to_points();
            let height = page.size[1].to_points();

            self.writer.write_obj(id, Page::new(self.offsets.page_tree)
                .media_box(Rect::new(0.0, 0.0, width, height))
                .contents(self.offsets.contents.0 ..= self.offsets.contents.1)
            )?;

            id += 1;
        }

        Ok(())
    }

    /// Write the page contents.
    fn write_contents(&mut self) -> PdfResult<()> {
        let mut id = self.offsets.contents.0;
        for page in &self.doc.pages {
            for content in &page.contents {
                self.writer.write_obj(id, &Text::new()
                    .set_font(1, 13.0)
                    .move_line(108.0, 734.0)
                    .write_text(&self.encode(&content.0))
                    .to_stream()
                )?;
                id += 1;
            }
        }

        Ok(())
    }

    /// Write the fonts.
    fn write_fonts(&mut self) -> PdfResult<()> {
        let id = self.offsets.fonts.0;

        self.writer.write_obj(id, &Type0Font::new(
            self.font.name.clone(),
            CMapEncoding::Predefined("Identity-H".to_owned()),
            id + 1
        )).unwrap();

        self.writer.write_obj(id + 1,
            CIDFont::new(
                CIDFontType::Type2,
                self.font.name.clone(),
                CIDSystemInfo::new("(Adobe)", "(Identity)", 0),
                id + 2,
            ).widths(vec![WidthRecord::start(0, self.font.widths.clone())])
        ).unwrap();

        self.writer.write_obj(id + 2,
            FontDescriptor::new(
                self.font.name.clone(),
                self.font.flags,
                self.font.italic_angle,
            )
            .font_bbox(self.font.bounding_box)
            .ascent(self.font.ascender)
            .descent(self.font.descender)
            .cap_height(self.font.cap_height)
            .stem_v(self.font.stem_v)
            .font_file_3(id + 3)
        ).unwrap();


        self.writer.write_obj(id + 3, &EmbeddedFont::OpenType(&self.font.data)).unwrap();

        Ok(())
    }

    /// Encode the given text for our font.
    fn encode(&self, text: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * text.len());
        for glyph in text.chars().map(|c| self.font.map(c)) {
            bytes.push((glyph >> 8) as u8);
            bytes.push((glyph & 0xff) as u8);
        }
        bytes
    }
}


/// The data we need from the font.
struct PdfFont {
    data: Vec<u8>,
    mapping: HashMap<char, u16>,
    default_glyph: u16,
    name: String,
    widths: Vec<GlyphUnit>,
    flags: FontFlags,
    italic_angle: f32,
    bounding_box: Rect<GlyphUnit>,
    ascender: GlyphUnit,
    descender: GlyphUnit,
    cap_height: GlyphUnit,
    stem_v: GlyphUnit,
}

impl PdfFont {
    /// Create a subetted version of the font and calculate some information
    /// needed for creating the _PDF_.
    pub fn new(font_name: &str, data: Vec<u8>, chars: HashSet<char>) -> PdfResult<PdfFont> {
        let mut readable = Cursor::new(&data);
        let mut reader = OpenTypeReader::new(&mut readable);

        let head = reader.read_table::<tables::Header>()?;
        let name = reader.read_table::<tables::Name>()?;
        let post = reader.read_table::<tables::Post>()?;
        let os2 = reader.read_table::<tables::OS2>()?;

        let font = Font::new(data);
        let (subsetted, mapping) = font.subsetted(
            chars,
            &["head", "hhea", "maxp", "hmtx", "loca", "glyf"],
            &["cvt ", "prep", "fpgm", "OS/2", "cmap", "name", "post"],
        )?;

        let unit_ratio = 1000.0 / (head.units_per_em as f32);
        let convert = |x| (unit_ratio * x as f32).round() as GlyphUnit;

        let base_font = name.get_decoded(NameEntry::PostScriptName);
        let font_name =  base_font.unwrap_or_else(|| font_name.to_owned());


        let mut flags = FontFlags::empty();
        flags.set(FontFlags::FIXED_PITCH, post.is_fixed_pitch);
        flags.set(FontFlags::SERIF, font_name.contains("Serif"));
        flags.insert(FontFlags::SYMBOLIC);
        flags.set(FontFlags::ITALIC, head.mac_style.contains(MacStyleFlags::ITALIC));
        flags.insert(FontFlags::SMALL_CAP);

        let mut readable = Cursor::new(&subsetted);
        let mut reader = OpenTypeReader::new(&mut readable);
        let hmtx = reader.read_table::<tables::HorizontalMetrics>()?;
        let widths = hmtx.metrics.iter().map(|m| convert(m.advance_width)).collect();


        Ok(PdfFont {
            data: subsetted,
            mapping,
            default_glyph: os2.us_default_char.unwrap_or(0),
            name: font_name,
            widths,
            flags,
            italic_angle: post.italic_angle.to_f32(),
            bounding_box: Rect::new(
                convert(head.x_min),
                convert(head.y_min),
                convert(head.x_max),
                convert(head.y_max)
            ),
            ascender: convert(os2.s_typo_ascender),
            descender: convert(os2.s_typo_descender),
            cap_height: convert(os2.s_cap_height.unwrap_or(os2.s_typo_ascender)),
            stem_v: (10.0 + 220.0 * (os2.us_weight_class as f32 - 50.0) / 900.0) as GlyphUnit,
        })
    }

    /// Map a character to it's glyph index.
    fn map(&self, c: char) -> u16 {
        self.mapping.get(&c).map(|&g| g).unwrap_or(self.default_glyph)
    }
}


#[cfg(test)]
mod pdf_tests {
    use super::*;
    use crate::parsing::ParseTree;
    use crate::doc::Generate;

    /// Create a pdf with a name from the source code.
    fn test(name: &str, src: &str) {
        let doc = src.parse_tree().unwrap().generate().unwrap();
        let path = format!("../target/typeset-pdf-{}.pdf", name);
        let mut file = std::fs::File::create(path).unwrap();
        file.write_pdf(&doc).unwrap();
    }

    #[test]
    fn pdf_simple() {
        test("unicode", "∑mbe∂∂ed font with Unicode!");
        test("parentheses", "Text with ) and ( or (enclosed) works.");
        test("multiline","
             Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed
             diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed
             diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum.
             Stet clita kasd gubergren, no sea takimata sanctus est.
        ");
    }

    #[test]
    fn pdf_composite_glyph() {
        test("composite-glyph", "Composite character‼");
    }
}

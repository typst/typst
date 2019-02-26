//! Writing of documents in the _PDF_ format.

use std::fmt;
use std::io::{self, Write, Cursor};
use crate::doc::Document;
use pdf::{PdfWriter, Id, Rect, Version, Trailer};
use pdf::doc::{DocumentCatalog, PageTree, Page, PageData, Resource, Content};
use pdf::text::Text;
use pdf::font::{Type0Font, CMapEncoding, CIDFont, CIDFontType, CIDSystemInfo,
               WidthRecord, FontDescriptor, EmbeddedFont, GlyphUnit};
use opentype::{OpenTypeReader, tables};


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

impl fmt::Display for PdfWritingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pdf writing error: {}", self.message)
    }
}


/// Shortcut macro to create bitflags from bools.
macro_rules! flags {
    ($($bit:expr => $value:expr),*) => {{
        let mut flags = 0;
        $(
            flags |= if $value { 1 << ($bit - 1) } else { 0 };
        )*
        flags
    }};
    ($($bit:expr => $value:expr,)*) => (flags!($($bit => $value),*));
}

/// Keeps track of the document while letting the pdf writer
/// generate the _PDF_.
struct PdfCreator<'a, W: Write> {
    writer: PdfWriter<'a, W>,
    doc: &'a Document,
    offsets: Offsets,
    font_data: FontData,
}

/// Offsets for the various groups of ids.
struct Offsets {
    catalog: Id,
    page_tree: Id,
    pages: (Id, Id),
    contents: (Id, Id),
    fonts: (Id, Id),
}

/// The data we need from the font.
struct FontData {
    data: Vec<u8>,
    name: tables::Name,
    head: tables::Header,
    post: tables::Post,
    os2: tables::OS2,
    hmtx: tables::HorizontalMetrics,
    cmap: tables::CharMap,
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

        // Read the font from a file.
        let data = std::fs::read(format!("../fonts/{}.ttf", doc.font))?;
        let font_data = FontData::load(data)?;

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
            font_data,
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
        self.writer.write_trailer(&Trailer {
            root: self.offsets.catalog,
        })?;

        Ok(self.writer.written())
    }

    /// Write the document catalog, page tree and pages.
    fn write_pages(&mut self) -> PdfResult<()> {
        // The document catalog
        self.writer.write_obj(self.offsets.catalog, &DocumentCatalog {
            page_tree: self.offsets.page_tree,
        })?;

        // Root page tree
        self.writer.write_obj(self.offsets.page_tree, &PageTree {
            parent: None,
            kids: (self.offsets.pages.0 ..= self.offsets.pages.1).collect(),
            data: PageData {
                resources: Some(vec![Resource::Font { nr: 1, id: self.offsets.fonts.0 }]),
                .. PageData::none()
            },
        })?;

        // The page objects
        let mut id = self.offsets.pages.0;
        for page in &self.doc.pages {
            let width = page.size[0].to_points();
            let height = page.size[1].to_points();

            let contents = (self.offsets.contents.0 ..= self.offsets.contents.1).collect();
            self.writer.write_obj(id, &Page {
                parent: self.offsets.page_tree,
                data: PageData {
                    media_box: Some(Rect::new(0.0, 0.0, width, height)),
                    contents: Some(contents),
                    .. PageData::none()
                },
            })?;

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
        let font_data = &self.font_data;

        // Create conversion function from font units to PDF units.
        let ratio = 1000.0 / (font_data.head.units_per_em as f32);
        let convert = |x| (ratio * x as f32).round() as GlyphUnit;

        let base_font = font_data.name.post_script_name.as_ref()
            .unwrap_or(&self.doc.font);

        self.writer.write_obj(id, &Type0Font {
            base_font: base_font.clone(),
            encoding: CMapEncoding::Predefined("Identity-H".to_owned()),
            descendant_font: id + 1,
            to_unicode: None,
        }).unwrap();

        self.writer.write_obj(id + 1, &CIDFont {
            subtype: CIDFontType::Type2,
            base_font: base_font.clone(),
            cid_system_info: CIDSystemInfo {
                registry: "(Adobe)".to_owned(),
                ordering: "(Identity)".to_owned(),
                supplement: 0,
            },
            font_descriptor: id + 2,
            widths: Some(vec![WidthRecord::Start(0,
                font_data.hmtx.metrics.iter()
                    .map(|m| convert(m.advance_width))
                    .collect::<Vec<_>>()
            )]),
            cid_to_gid_map: Some(CMapEncoding::Predefined("Identity".to_owned())),
        }).unwrap();

        self.writer.write_obj(id + 2, &FontDescriptor {
            font_name: base_font.clone(),
            flags: flags!(
                1 => font_data.post.is_fixed_pitch,
                2 => base_font.contains("Serif"),
                3 => true, 4 => false, 6 => false,
                7 => (font_data.head.mac_style & 1) != 0,
                17 => false, 18 => true, 19 => false,
            ),
            found_bbox: Rect::new(
                convert(font_data.head.x_min),
                convert(font_data.head.y_min),
                convert(font_data.head.x_max),
                convert(font_data.head.y_max)
            ),
            italic_angle: font_data.post.italic_angle.to_f32(),
            ascent: convert(font_data.os2.s_typo_ascender),
            descent: convert(font_data.os2.s_typo_descender),
            cap_height: convert(font_data.os2.s_cap_height
                .unwrap_or(font_data.os2.s_typo_ascender)),
            stem_v: (10.0 + 220.0 *
                (font_data.os2.us_weight_class as f32 - 50.0) / 900.0) as GlyphUnit,
            font_file_3: Some(id + 3),
        }).unwrap();

        self.writer.write_obj(id + 3, &EmbeddedFont::OpenType(&font_data.data)).unwrap();

        Ok(())
    }

    /// Encode the given text for our font.
    fn encode(&self, text: &str) -> Vec<u8> {
        let default = self.font_data.os2.us_default_char.unwrap_or(0);
        let mut bytes = Vec::with_capacity(2 * text.len());
        text.chars().map(|c| {
            self.font_data.cmap.get(c).unwrap_or(default)
        })
        .for_each(|glyph| {
            bytes.push((glyph >> 8) as u8);
            bytes.push((glyph & 0xff) as u8);
        });
        bytes
    }
}

impl FontData {
    /// Load various needed tables from the font data.
    pub fn load(data: Vec<u8>) -> PdfResult<FontData> {
        let mut readable = Cursor::new(data);
        let mut reader = OpenTypeReader::new(&mut readable);

        let name = reader.read_table::<tables::Name>()?;
        let head = reader.read_table::<tables::Header>()?;
        let post = reader.read_table::<tables::Post>()?;
        let os2 = reader.read_table::<tables::OS2>()?;
        let hmtx = reader.read_table::<tables::HorizontalMetrics>()?;
        let cmap = reader.read_table::<tables::CharMap>()?;

        Ok(FontData {
            data: readable.into_inner(),
            name, head, post, os2, hmtx, cmap,
        })
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
}

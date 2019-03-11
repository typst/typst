//! Writing of documents in the _PDF_ format.

use std::fmt;
use std::io::{self, Write, Cursor};
use std::collections::HashSet;
use pdf::{PdfWriter, Id, Rect, Version, Trailer};
use pdf::doc::{Catalog, PageTree, Page, Resource, Content};
use pdf::text::Text;
use pdf::font::{
    Type0Font, CMapEncoding, CIDFont, CIDFontType, CIDSystemInfo,
    WidthRecord, FontDescriptor, FontFlags, EmbeddedFont, GlyphUnit
};
use opentype::{OpenTypeReader, tables::{self, MacStyleFlags}};
use crate::doc::{self, Document, TextCommand};
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
    fonts: Vec<PdfFont>,
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
        let content_count = doc.pages.iter().flat_map(|p| p.text.iter()).count() as Id;
        let contents = (pages.1 + 1, pages.1 + content_count);
        let fonts = (contents.1 + 1, contents.1 + 4 * doc.fonts.len() as Id);

        let offsets = Offsets {
            catalog,
            page_tree,
            pages,
            contents,
            fonts,
        };

        assert!(doc.fonts.len() > 0);

        // Find out which chars are used in this document.
        let mut char_sets = vec![HashSet::new(); doc.fonts.len()];
        let mut current_font: usize = 0;
        for page in &doc.pages {
            for text in &page.text {
                for command in &text.commands {
                    match command {
                        TextCommand::Text(string) => {
                            char_sets[current_font].extend(string.chars());
                        },
                        TextCommand::SetFont(id, _) => {
                            assert!(*id < doc.fonts.len());
                            current_font = *id;
                        },
                        _ => {},
                    }
                }
            }
        }

        // Create a subsetted pdf font.
        let fonts = doc.fonts.iter().enumerate().map(|(i, font)| {
            PdfFont::new(font, &char_sets[i])
        }).collect::<PdfResult<Vec<_>>>()?;

        Ok(PdfCreator {
            writer: PdfWriter::new(target),
            doc,
            offsets,
            fonts,
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
            let width = page.width.to_points();
            let height = page.height.to_points();

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
            for text in &page.text {
                self.write_text(id, text)?;
                id += 1;
            }
        }
        Ok(())
    }

    fn write_text(&mut self, id: u32, text: &doc::Text) -> PdfResult<()> {
        let mut current_font = 0;
        let encoded = text.commands.iter().filter_map(|cmd| match cmd {
            TextCommand::Text(string) => Some(self.fonts[current_font].encode(&string)),
            TextCommand::SetFont(id, _) => { current_font = *id; None },
            _ => None,
        }).collect::<Vec<_>>();

        let mut object = Text::new();
        let mut nr = 0;

        for command in &text.commands {
            match command {
                TextCommand::Text(_) => {
                    object.write_text(&encoded[nr]);
                    nr += 1;
                },
                TextCommand::SetFont(id, size) => {
                    object.set_font(*id as u32 + 1, *size);
                },
                TextCommand::Move(x, y) => {
                    object.move_line(x.to_points(), y.to_points());
                }
            }
        }

        self.writer.write_obj(id, &object.to_stream())?;

        Ok(())
    }

    /// Write the fonts.
    fn write_fonts(&mut self) -> PdfResult<()> {
        let mut id = self.offsets.fonts.0;

        for font in &self.fonts {
            self.writer.write_obj(id, &Type0Font::new(
                font.name.clone(),
                CMapEncoding::Predefined("Identity-H".to_owned()),
                id + 1
            ))?;

            self.writer.write_obj(id + 1,
                CIDFont::new(
                    CIDFontType::Type2,
                    font.name.clone(),
                    CIDSystemInfo::new("(Adobe)", "(Identity)", 0),
                    id + 2,
                ).widths(vec![WidthRecord::start(0, font.widths.clone())])
            )?;

            self.writer.write_obj(id + 2,
                FontDescriptor::new(
                    font.name.clone(),
                    font.flags,
                    font.italic_angle,
                )
                .font_bbox(font.bounding_box)
                .ascent(font.ascender)
                .descent(font.descender)
                .cap_height(font.cap_height)
                .stem_v(font.stem_v)
                .font_file_3(id + 3)
            )?;

            self.writer.write_obj(id + 3, &EmbeddedFont::OpenType(&font.program))?;

            id += 4;
        }

        Ok(())
    }
}


/// The data we need from the font.
struct PdfFont {
    font: Font,
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
    pub fn new(font: &Font, chars: &HashSet<char>) -> PdfResult<PdfFont> {
        let mut readable = Cursor::new(&font.program);
        let mut reader = OpenTypeReader::new(&mut readable);

        let head = reader.read_table::<tables::Header>()?;
        let post = reader.read_table::<tables::Post>()?;
        let os2 = reader.read_table::<tables::OS2>()?;

        let subsetted = font.subsetted(
            chars.iter().cloned(),
            &["head", "hhea", "maxp", "hmtx", "loca", "glyf"],
            &["cvt ", "prep", "fpgm", "OS/2", "cmap", "name", "post"],
        )?;

        let mut flags = FontFlags::empty();
        flags.set(FontFlags::FIXED_PITCH, post.is_fixed_pitch);
        flags.set(FontFlags::SERIF, font.name.contains("Serif"));
        flags.insert(FontFlags::SYMBOLIC);
        flags.set(FontFlags::ITALIC, head.mac_style.contains(MacStyleFlags::ITALIC));
        flags.insert(FontFlags::SMALL_CAP);

        let widths = subsetted.widths.iter()
            .map(|w| (1000.0 * w.to_points()).round() as GlyphUnit)
            .collect();

        let unit_ratio = 1.0 / (head.units_per_em as f32);
        let convert = |x| (unit_ratio * x as f32).round() as GlyphUnit;

        Ok(PdfFont {
            font: subsetted,
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
}

impl std::ops::Deref for PdfFont {
    type Target = Font;

    fn deref(&self) -> &Font {
        &self.font
    }
}


#[cfg(test)]
mod pdf_tests {
    use super::*;
    use crate::parsing::ParseTree;
    use crate::engine::Typeset;

    /// Create a pdf with a name from the source code.
    fn test(name: &str, src: &str) {
        let doc = src.parse_tree().unwrap().typeset().unwrap();
        let path = format!("../target/typeset-pdf-{}.pdf", name);
        let mut file = std::fs::File::create(path).unwrap();
        file.write_pdf(&doc).unwrap();
    }

    #[test]
    fn pdf() {
        test("unicode", "∑mbe∂∂ed font with Unicode!");
        test("parentheses", "Text with ) and ( or (enclosed) works.");
        test("composite-glyph", "Composite character‼");
        test("multiline","
             Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed
             diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed
             diam voluptua. At vero eos et accusam et justo duo dolores et ea rebum.
             Stet clita kasd gubergren, no sea takimata sanctus est.
        ");
    }
}

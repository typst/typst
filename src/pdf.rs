//! Writing of documents in the _PDF_ format.

use std::collections::HashSet;
use std::error;
use std::fmt;
use std::io::{self, Write};
use pdf::{PdfWriter, Reference, Rect, Version, Trailer, DocumentCatalog};
use pdf::{PageTree, Page, Resource, Text, Content};
use pdf::font::{Type0Font, CMapEncoding, CIDFont, CIDFontType, CIDSystemInfo};
use pdf::font::{WidthRecord, FontDescriptor, FontFlags, EmbeddedFont, GlyphUnit};
use crate::doc::{Document, Size, Text as DocText, TextCommand as DocTextCommand};
use crate::font::{Font, FontError};


/// Writes documents in the _PDF_ format.
pub struct PdfCreator<'a, W: Write> {
    writer: PdfWriter<'a, W>,
    doc: &'a Document,
    offsets: Offsets,
    fonts: Vec<PdfFont>,
}

/// Offsets for the various groups of ids.
struct Offsets {
    catalog: Reference,
    page_tree: Reference,
    pages: (Reference, Reference),
    contents: (Reference, Reference),
    fonts: (Reference, Reference),
}

impl<'a, W: Write> PdfCreator<'a, W> {
    /// Create a new _PDF_ Creator.
    pub fn new(doc: &'a Document, target: &'a mut W) -> PdfResult<PdfCreator<'a, W>> {
        // Calculate a unique id for all object to come
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages = (page_tree + 1, page_tree + doc.pages.len() as Reference);
        let content_count = doc.pages.iter().flat_map(|p| p.text.iter()).count() as Reference;
        let contents = (pages.1 + 1, pages.1 + content_count);
        let fonts = (contents.1 + 1, contents.1 + 4 * doc.fonts.len() as Reference);

        let offsets = Offsets {
            catalog,
            page_tree,
            pages,
            contents,
            fonts,
        };

        // Find out which chars are used in this document.
        let mut char_sets = vec![HashSet::new(); doc.fonts.len()];
        let mut current_font: usize = 0;
        for page in &doc.pages {
            for text in &page.text {
                for command in &text.commands {
                    match command {
                        DocTextCommand::Text(string)
                          => char_sets[current_font].extend(string.chars()),
                        DocTextCommand::SetFont(id, _) => current_font = *id,
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
    pub fn write(&mut self) -> PdfResult<usize> {
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
        self.writer.write_obj(self.offsets.catalog,
            &DocumentCatalog::new(self.offsets.page_tree))?;

        // Root page tree
        self.writer.write_obj(self.offsets.page_tree, PageTree::new()
            .kids(self.offsets.pages.0 ..= self.offsets.pages.1)
            .resource(Resource::Font { nr: 1, id: self.offsets.fonts.0 })
        )?;

        // The page objects
        let mut id = self.offsets.pages.0;
        for page in &self.doc.pages {
            self.writer.write_obj(id, Page::new(self.offsets.page_tree)
                .media_box(Rect::new(
                    0.0, 0.0,
                    page.width.to_points(), page.height.to_points())
                )
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

    fn write_text(&mut self, id: u32, text: &DocText) -> PdfResult<()> {
        let mut current_font = 0;
        let encoded = text.commands.iter().filter_map(|cmd| match cmd {
            DocTextCommand::Text(string) => Some(self.fonts[current_font].encode(&string)),
            DocTextCommand::SetFont(id, _) => { current_font = *id; None },
            _ => None,
        }).collect::<Vec<_>>();

        let mut object = Text::new();
        let mut nr = 0;

        for command in &text.commands {
            match command {
                DocTextCommand::Text(_) => {
                    object.write_text(&encoded[nr]);
                    nr += 1;
                },
                DocTextCommand::SetFont(id, size) => { object.set_font(*id as u32 + 1, *size); },
                DocTextCommand::Move(x, y) => { object.move_line(x.to_points(), y.to_points()); },
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
        // Subset the font using the selected characters
        let subsetted = font.subsetted(
            chars.iter().cloned(),
            &["head", "hhea", "maxp", "hmtx", "loca", "glyf"],
            &["cvt ", "prep", "fpgm", /* "OS/2", "cmap", "name", "post" */],
        )?;

        // Specify flags for the font
        let mut flags = FontFlags::empty();
        flags.set(FontFlags::FIXED_PITCH, font.metrics.is_fixed_pitch);
        flags.set(FontFlags::SERIF, font.name.contains("Serif"));
        flags.insert(FontFlags::SYMBOLIC);
        flags.set(FontFlags::ITALIC, font.metrics.is_italic);
        flags.insert(FontFlags::SMALL_CAP);

        // Transform the widths
        let widths = subsetted.widths.iter().map(|&x| size_to_glyph_unit(x)).collect();

        Ok(PdfFont {
            font: subsetted,
            widths,
            flags,
            italic_angle: font.metrics.italic_angle,
            bounding_box: Rect::new(
                size_to_glyph_unit(font.metrics.bounding_box[0]),
                size_to_glyph_unit(font.metrics.bounding_box[1]),
                size_to_glyph_unit(font.metrics.bounding_box[2]),
                size_to_glyph_unit(font.metrics.bounding_box[3]),
            ),
            ascender: size_to_glyph_unit(font.metrics.ascender),
            descender: size_to_glyph_unit(font.metrics.descender),
            cap_height: size_to_glyph_unit(font.metrics.cap_height),
            stem_v: (10.0 + 0.244 * (font.metrics.weight_class as f32 - 50.0)) as GlyphUnit,
        })
    }
}

/// Convert a size into a _PDF_ glyph unit.
fn size_to_glyph_unit(size: Size) -> GlyphUnit {
    (1000.0 * size.to_points()).round() as GlyphUnit
}

impl std::ops::Deref for PdfFont {
    type Target = Font;

    fn deref(&self) -> &Font {
        &self.font
    }
}

/// Result type used for parsing.
type PdfResult<T> = std::result::Result<T, PdfError>;

/// The error type for _PDF_ creation.
pub enum PdfError {
    /// An error occured while subsetting the font for the _PDF_.
    Font(FontError),
    /// An I/O Error on the underlying writable occured.
    Io(io::Error),
}

impl error::Error for PdfError {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            PdfError::Font(err) => Some(err),
            PdfError::Io(err) => Some(err),
        }
    }
}

impl fmt::Display for PdfError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PdfError::Font(err) => write!(f, "font error: {}", err),
            PdfError::Io(err) => write!(f, "io error: {}", err),
        }
    }
}

impl fmt::Debug for PdfError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<io::Error> for PdfError {
    #[inline]
    fn from(err: io::Error) -> PdfError {
        PdfError::Io(err)
    }
}

impl From<FontError> for PdfError {
    #[inline]
    fn from(err: FontError) -> PdfError {
        PdfError::Font(err)
    }
}

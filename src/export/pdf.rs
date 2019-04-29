//! Exporting into _PDF_ documents.

use std::collections::HashSet;
use std::io::{self, Write};

use pdf::{PdfWriter, Ref, Rect, Version, Trailer, Content};
use pdf::doc::{Catalog, PageTree, Page, Resource, Text};
use pdf::font::{Type0Font, CIDFont, CIDFontType, CIDSystemInfo, FontDescriptor, FontFlags};
use pdf::font::{GlyphUnit, CMap, CMapEncoding, WidthRecord, FontStream};

use crate::doc::{Document, Text as DocText, TextCommand};
use crate::font::{Font, FontError};
use crate::engine::Size;


/// Exports documents into _PDFs_.
pub struct PdfExporter {}

impl PdfExporter {
    /// Create a new exporter.
    #[inline]
    pub fn new() -> PdfExporter {
        PdfExporter {}
    }

    /// Export a typesetted document into a writer. Returns how many bytes were written.
    #[inline]
    pub fn export<W: Write>(&self, document: &Document, target: W) -> PdfResult<usize> {
        let mut engine = PdfEngine::new(document, target)?;
        engine.write()
    }
}

/// Writes documents in the _PDF_ format.
struct PdfEngine<'d, W: Write> {
    writer: PdfWriter<W>,
    doc: &'d Document,
    offsets: Offsets,
    fonts: Vec<PdfFont>,
}

/// Offsets for the various groups of ids.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Offsets {
    catalog: Ref,
    page_tree: Ref,
    pages: (Ref, Ref),
    contents: (Ref, Ref),
    fonts: (Ref, Ref),
}

impl<'d, W: Write> PdfEngine<'d, W> {
    /// Create a new _PDF_ Creator.
    fn new(doc: &'d Document, target: W) -> PdfResult<PdfEngine<'d, W>> {
        // Calculate a unique id for all objects that will be written.
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages = (page_tree + 1, page_tree + doc.pages.len() as Ref);
        let content_count = doc.pages.iter().flat_map(|p| p.text.iter()).count() as Ref;
        let contents = (pages.1 + 1, pages.1 + content_count);
        let fonts = (contents.1 + 1, contents.1 + 5 * doc.fonts.len() as Ref);
        let offsets = Offsets { catalog, page_tree, pages, contents, fonts };

        // Create a subsetted PDF font for each font in the document.
        let fonts = {
            let mut font = 0usize;
            let mut chars = vec![HashSet::new(); doc.fonts.len()];

            // Iterate through every text object on every page and find out
            // which characters they use.
            for text in doc.pages.iter().flat_map(|page| page.text.iter()) {
                for command in &text.commands {
                    match command {
                        TextCommand::Text(string) => chars[font].extend(string.chars()),
                        TextCommand::SetFont(id, _) => font = *id,
                        _ => {},
                    }
                }
            }

            doc.fonts.iter()
                .enumerate()
                .map(|(i, font)| PdfFont::new(font, &chars[i]))
                .collect::<PdfResult<Vec<_>>>()?
        };

        Ok(PdfEngine {
            writer: PdfWriter::new(target),
            doc,
            offsets,
            fonts,
        })
    }

    /// Write the complete document.
    fn write(&mut self) -> PdfResult<usize> {
        // Write all the things!
        self.writer.write_header(&Version::new(1, 7))?;
        self.write_pages()?;
        self.write_contents()?;
        self.write_fonts()?;
        self.writer.write_xref_table()?;
        self.writer.write_trailer(&Trailer::new(self.offsets.catalog))?;
        Ok(self.writer.written())
    }

    /// Write the document catalog and page tree.
    fn write_pages(&mut self) -> PdfResult<()> {
        // The document catalog.
        self.writer.write_obj(self.offsets.catalog, &Catalog::new(self.offsets.page_tree))?;

        // The font resources.
        let fonts = (0 .. self.fonts.len())
            .map(|i| Resource::Font((i + 1) as u32, self.offsets.fonts.0 + 5 * i as u32));

        // The root page tree.
        self.writer.write_obj(self.offsets.page_tree, PageTree::new()
            .kids(ids(self.offsets.pages))
            .resources(fonts)
        )?;

        // The page objects.
        for (id, page) in ids(self.offsets.pages).zip(&self.doc.pages) {
            self.writer.write_obj(id, Page::new(self.offsets.page_tree)
                .media_box(Rect::new(0.0, 0.0, page.width.to_points(), page.height.to_points()))
                .contents(ids(self.offsets.contents))
            )?;
        }

        Ok(())
    }

    /// Write the contents of all pages.
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

    /// Write one text object.
    fn write_text(&mut self, id: u32, doc_text: &DocText) -> PdfResult<()> {
        let mut font = 0;
        let mut text = Text::new();

        for command in &doc_text.commands {
            match command {
                TextCommand::Text(string) => { text.tj(self.fonts[font].encode(&string)); },
                TextCommand::Move(x, y) => { text.td(x.to_points(), y.to_points()); },
                TextCommand::SetFont(id, size) => {
                    font = *id;
                    text.tf(*id as u32 + 1, *size);
                },
            }
        }

        self.writer.write_obj(id, &text.to_stream())?;

        Ok(())
    }

    /// Write all the fonts.
    fn write_fonts(&mut self) -> PdfResult<()> {
        let mut id = self.offsets.fonts.0;

        for font in &self.fonts {
            let base_font = format!("ABCDEF+{}", font.name);

            // Write the base font object referencing the CID font.
            self.writer.write_obj(id,
                Type0Font::new(
                    base_font.clone(),
                    CMapEncoding::Predefined("Identity-H".to_owned()),
                    id + 1
                ).to_unicode(id + 3)
            )?;

            let system_info = CIDSystemInfo::new("Adobe", "Identity", 0);

            // Write the CID font referencing the font descriptor.
            self.writer.write_obj(id + 1,
                CIDFont::new(
                    CIDFontType::Type2,
                    base_font.clone(),
                    system_info.clone(),
                    id + 2,
                ).widths(vec![WidthRecord::start(0, font.widths.clone())])
            )?;

            // Write the font descriptor (contains the global information about the font).
            self.writer.write_obj(id + 2,
                FontDescriptor::new(
                    base_font,
                    font.flags,
                    font.italic_angle,
                )
                .font_bbox(font.bounding_box)
                .ascent(font.ascender)
                .descent(font.descender)
                .cap_height(font.cap_height)
                .stem_v(font.stem_v)
                .font_file_2(id + 4)
            )?;

            // The CMap, which maps glyphs to unicode codepoints.
            let mapping = font.font.mapping.iter().map(|(&c, &cid)| (cid, c));
            self.writer.write_obj(id + 3, &CMap::new("Custom", system_info, mapping))?;

            // Finally write the subsetted font program.
            self.writer.write_obj(id + 4, &FontStream::new(&font.program))?;

            id += 5;
        }

        Ok(())
    }
}

/// Create an iterator from reference pair.
fn ids((start, end): (Ref, Ref)) -> impl Iterator<Item=Ref> {
    start ..= end
}

/// The data we need from the font.
#[derive(Debug, Clone)]
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
    fn new(font: &Font, chars: &HashSet<char>) -> PdfResult<PdfFont> {
        /// Convert a size into a _PDF_ glyph unit.
        fn size_to_glyph_unit(size: Size) -> GlyphUnit {
            (1000.0 * size.to_points()).round() as GlyphUnit
        }

        // Subset the font using the selected characters
        let subsetted = font.subsetted(
            chars.iter().cloned(),
            &["head", "hhea", "maxp", "hmtx", "loca", "glyf"][..],
            &["cvt ", "prep", "fpgm", /* "OS/2", "cmap", "name", "post" */][..],
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

impl std::ops::Deref for PdfFont {
    type Target = Font;

    fn deref(&self) -> &Font {
        &self.font
    }
}

/// The error type for _PDF_ creation.
pub enum PdfExportError {
    /// An error occured while subsetting the font for the _PDF_.
    Font(FontError),
    /// An I/O Error on the underlying writable occured.
    Io(io::Error),
}

error_type! {
    err: PdfExportError,
    res: PdfResult,
    show: f => match err {
        PdfExportError::Font(err) => write!(f, "font error: {}", err),
        PdfExportError::Io(err) => write!(f, "io error: {}", err),
    },
    source: match err {
        PdfExportError::Font(err) => Some(err),
        PdfExportError::Io(err) => Some(err),
    },
    from: (io::Error, PdfExportError::Io(err)),
    from: (FontError, PdfExportError::Font(err)),
}

//! Exporting into _PDF_ documents.

use std::collections::{HashMap, HashSet};
use std::io::{self, Write};

use pdf::{PdfWriter, Ref, Rect, Version, Trailer, Content};
use pdf::doc::{Catalog, PageTree, Page, Resource, Text};
use pdf::font::{Type0Font, CIDFont, CIDFontType, CIDSystemInfo, FontDescriptor, FontFlags};
use pdf::font::{GlyphUnit, CMap, CMapEncoding, WidthRecord, FontStream};

use crate::doc::{Document, Page as DocPage, LayoutAction};
use crate::font::{Font, FontLoader, FontError};
use crate::size::{Size, Size2D};


/// Exports documents into _PDFs_.
#[derive(Debug)]
pub struct PdfExporter {}

impl PdfExporter {
    /// Create a new exporter.
    #[inline]
    pub fn new() -> PdfExporter {
        PdfExporter {}
    }

    /// Export a typesetted document into a writer. Returns how many bytes were written.
    #[inline]
    pub fn export<W: Write>(&self, document: &Document, loader: &FontLoader, target: W)
    -> PdfResult<usize> {
        let mut engine = PdfEngine::new(document, loader, target)?;
        engine.write()
    }
}

/// Writes documents in the _PDF_ format.
#[derive(Debug)]
struct PdfEngine<'d, W: Write> {
    writer: PdfWriter<W>,
    doc: &'d Document,
    offsets: Offsets,
    font_remap: HashMap<usize, usize>,
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
    /// Create a new _PDF_ engine.
    fn new(doc: &'d Document, loader: &FontLoader, target: W) -> PdfResult<PdfEngine<'d, W>> {
        // Create a subsetted PDF font for each font in the document.
        let mut font_remap = HashMap::new();
        let fonts = {
            let mut font = 0usize;
            let mut chars = HashMap::new();

            // Find out which characters are used for each font.
            for page in &doc.pages {
                for action in &page.actions {
                    match action {
                        LayoutAction::WriteText(string) => {
                            chars.entry(font)
                                .or_insert_with(HashSet::new)
                                .extend(string.chars())
                        },
                        LayoutAction::SetFont(id, _) => {
                            font = *id;
                            let new_id = font_remap.len();
                            font_remap.entry(font).or_insert(new_id);
                        },
                        _ => {},
                    }
                }
            }

            // Collect the fonts into a vector in the order of the values in the remapping.
            let mut order = font_remap.iter().map(|(&old, &new)| (old, new)).collect::<Vec<_>>();
            order.sort_by_key(|&(_, new)| new);
            order.into_iter()
                .map(|(old, _)| PdfFont::new(&loader.get_with_index(old), &chars[&old]))
                .collect::<PdfResult<Vec<_>>>()?
        };

        // Calculate a unique id for all objects that will be written.
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages = (page_tree + 1, page_tree + doc.pages.len() as Ref);
        let contents = (pages.1 + 1, pages.1 + doc.pages.len() as Ref);
        let font_offsets = (contents.1 + 1, contents.1 + 5 * fonts.len() as Ref);
        let offsets = Offsets { catalog, page_tree, pages, contents, fonts: font_offsets };

        Ok(PdfEngine {
            writer: PdfWriter::new(target),
            doc,
            offsets,
            font_remap,
            fonts,
        })
    }

    /// Write the complete document.
    fn write(&mut self) -> PdfResult<usize> {
        self.writer.write_header(&Version::new(1, 7))?;
        self.write_page_tree()?;
        self.write_pages()?;
        self.write_fonts()?;
        self.writer.write_xref_table()?;
        self.writer.write_trailer(&Trailer::new(self.offsets.catalog))?;
        Ok(self.writer.written())
    }

    /// Write the document catalog and page tree.
    fn write_page_tree(&mut self) -> PdfResult<()> {
        // The document catalog
        self.writer.write_obj(self.offsets.catalog, &Catalog::new(self.offsets.page_tree))?;

        // The font resources
        let offset = self.offsets.fonts.0;
        let fonts = (0 .. self.fonts.len())
            .map(|i| Resource::Font((i + 1) as u32, offset + 5 * i as u32));

        // The root page tree
        self.writer.write_obj(self.offsets.page_tree, PageTree::new()
            .kids(ids(self.offsets.pages))
            .resources(fonts)
        )?;

        // The page objects
        for (id, page) in ids(self.offsets.pages).zip(&self.doc.pages) {
            self.writer.write_obj(id, Page::new(self.offsets.page_tree)
                .media_box(Rect::new(0.0, 0.0, page.width.to_pt(), page.height.to_pt()))
                .contents(ids(self.offsets.contents))
            )?;
        }

        Ok(())
    }

    /// Write the contents of all pages.
    fn write_pages(&mut self) -> PdfResult<()> {
        for (id, page) in ids(self.offsets.contents).zip(&self.doc.pages) {
            self.write_page(id, &page)?;
        }
        Ok(())
    }

    /// Write the content of a page.
    fn write_page(&mut self, id: u32, page: &DocPage) -> PdfResult<()> {
        let mut text = Text::new();
        let mut active_font = (std::usize::MAX, 0.0);

        // The last set position and font,
        // these only get flushed lazily when content is written.
        let mut next_pos = Some(Size2D::zero());
        let mut next_font = None;

        for action in &page.actions {
            match action {
                LayoutAction::MoveAbsolute(pos) => next_pos = Some(*pos),
                LayoutAction::SetFont(id, size) => next_font = Some((self.font_remap[id], *size)),
                LayoutAction::WriteText(string) => {
                    // Flush the font if it is different from the current.
                    if let Some((id, size)) = next_font {
                        if (id, size) != active_font {
                            text.tf(id as u32 + 1, size);
                            active_font = (id, size);
                            next_font = None;
                        }
                    }

                    // Flush the position.
                    if let Some(pos) = next_pos {
                        let x = pos.x.to_pt();
                        let y = (page.height - pos.y - Size::pt(active_font.1)).to_pt();
                        text.tm(1.0, 0.0, 0.0, 1.0, x, y);
                        next_pos = None;
                    }

                    // Write the text.
                    text.tj(self.fonts[active_font.0].encode_text(&string));
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

            // Write the CMap, which maps glyphs to unicode codepoints.
            let mapping = font.font.mapping.iter().map(|(&c, &cid)| (cid, c));
            self.writer.write_obj(id + 3, &CMap::new("Custom", system_info, mapping))?;

            // Finally write the subsetted font program.
            self.writer.write_obj(id + 4, &FontStream::new(&font.program))?;

            id += 5;
        }

        Ok(())
    }
}

/// Create an iterator from a reference pair.
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
            (1000.0 * size.to_pt()).round() as GlyphUnit
        }

        let subset_result = font.subsetted(
            chars.iter().cloned(),
            &["head", "hhea", "hmtx", "maxp", "cmap", "cvt ", "fpgm", "prep", "loca", "glyf"][..]
        );

        // Check if the subsetting was successful and if it could not handle this
        // font we just copy it plainly.
        let subsetted = match subset_result {
            Ok(font) => font,
            Err(FontError::UnsupportedFont(_)) => font.clone(),
            Err(err) => return Err(err.into()),
        };

        let mut flags = FontFlags::empty();
        flags.set(FontFlags::FIXED_PITCH, font.metrics.monospace);
        flags.set(FontFlags::SERIF, font.name.contains("Serif"));
        flags.insert(FontFlags::SYMBOLIC);
        flags.set(FontFlags::ITALIC, font.metrics.italic);
        flags.insert(FontFlags::SMALL_CAP);

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

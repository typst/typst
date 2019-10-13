//! Exporting into _PDF_ documents.

use std::collections::{HashMap, HashSet};
use std::io::{self, Write};

use tide::content::Content;
use tide::doc::{Catalog, Page, PageTree, Resource, Text};
use tide::font::{CIDFont, CIDFontType, CIDSystemInfo, FontDescriptor, FontFlags, Type0Font};
use tide::font::{CMap, CMapEncoding, FontStream, GlyphUnit, WidthRecord};
use tide::{PdfWriter, Rect, Ref, Trailer, Version};

use toddle::font::OwnedFont;
use toddle::query::SharedFontLoader;
use toddle::tables::{
    CharMap, Header, HorizontalMetrics, MacStyleFlags, Name, NameEntry, Post, OS2,
};
use toddle::Error as FontError;

use crate::layout::{Layout, LayoutAction, MultiLayout};
use crate::size::{Size, Size2D};

/// Exports layouts into _PDFs_.
#[derive(Debug)]
pub struct PdfExporter {}

impl PdfExporter {
    /// Create a new exporter.
    #[inline]
    pub fn new() -> PdfExporter {
        PdfExporter {}
    }

    /// Export a finished layouts into a writer. Returns how many bytes were
    /// written.
    #[inline]
    pub fn export<W: Write>(
        &self,
        layout: &MultiLayout,
        loader: &SharedFontLoader,
        target: W,
    ) -> PdfResult<usize>
    {
        let mut engine = PdfEngine::new(layout, loader, target)?;
        engine.write()
    }
}

/// Writes layouts in the _PDF_ format.
struct PdfEngine<'d, W: Write> {
    writer: PdfWriter<W>,
    layout: &'d MultiLayout,
    offsets: Offsets,
    font_remap: HashMap<usize, usize>,
    fonts: Vec<OwnedFont>,
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
    fn new(
        layout: &'d MultiLayout,
        loader: &SharedFontLoader,
        target: W,
    ) -> PdfResult<PdfEngine<'d, W>>
    {
        // Create a subsetted PDF font for each font in the layout.
        let mut font_remap = HashMap::new();
        let fonts = {
            let mut font = 0usize;
            let mut chars = HashMap::new();

            // Find out which characters are used for each font.
            for boxed in &layout.layouts {
                for action in &boxed.actions {
                    match action {
                        LayoutAction::WriteText(string) => chars
                            .entry(font)
                            .or_insert_with(HashSet::new)
                            .extend(string.chars()),
                        LayoutAction::SetFont(id, _) => {
                            font = *id;
                            let new_id = font_remap.len();
                            font_remap.entry(font).or_insert(new_id);
                        }
                        _ => {}
                    }
                }
            }

            // Collect the fonts into a vector in the order of the values in the remapping.
            let mut loader = loader.borrow_mut();
            let mut order = font_remap
                .iter()
                .map(|(&old, &new)| (old, new))
                .collect::<Vec<_>>();
            order.sort_by_key(|&(_, new)| new);

            let mut fonts = vec![];
            for (index, _) in order {
                let font = loader.get_with_index(index);
                let subsetted = font.subsetted(
                    chars[&index].iter().cloned(),
                    &[
                        "name", "OS/2", "post", "head", "hhea", "hmtx", "maxp", "cmap", "cvt ",
                        "fpgm", "prep", "loca", "glyf",
                    ][..],
                )?;
                fonts.push(OwnedFont::from_bytes(subsetted)?);
            }

            fonts
        };

        // Calculate a unique id for all objects that will be written.
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages = (page_tree + 1, page_tree + layout.layouts.len() as Ref);
        let contents = (pages.1 + 1, pages.1 + layout.layouts.len() as Ref);
        let font_offsets = (contents.1 + 1, contents.1 + 5 * fonts.len() as Ref);
        let offsets = Offsets {
            catalog,
            page_tree,
            pages,
            contents,
            fonts: font_offsets,
        };

        Ok(PdfEngine {
            writer: PdfWriter::new(target),
            layout,
            offsets,
            font_remap,
            fonts,
        })
    }

    /// Write the complete layout.
    fn write(&mut self) -> PdfResult<usize> {
        self.writer.write_header(Version::new(1, 7))?;
        self.write_page_tree()?;
        self.write_pages()?;
        self.write_fonts()?;
        self.writer.write_xref_table()?;
        self.writer
            .write_trailer(Trailer::new(self.offsets.catalog))?;
        Ok(self.writer.written())
    }

    /// Write the document catalog and page tree.
    fn write_page_tree(&mut self) -> PdfResult<()> {
        // The document catalog
        self.writer
            .write_obj(self.offsets.catalog, &Catalog::new(self.offsets.page_tree))?;

        // The font resources
        let offset = self.offsets.fonts.0;
        let fonts =
            (0..self.fonts.len()).map(|i| Resource::Font((i + 1) as u32, offset + 5 * i as u32));

        // The root page tree
        self.writer.write_obj(
            self.offsets.page_tree,
            PageTree::new()
                .kids(ids(self.offsets.pages))
                .resources(fonts),
        )?;

        // The page objects
        for (id, page) in ids(self.offsets.pages).zip(&self.layout.layouts) {
            let rect = Rect::new(
                0.0,
                0.0,
                page.dimensions.x.to_pt(),
                page.dimensions.y.to_pt(),
            );
            self.writer.write_obj(
                id,
                Page::new(self.offsets.page_tree)
                    .media_box(rect)
                    .contents(ids(self.offsets.contents)),
            )?;
        }

        Ok(())
    }

    /// Write the contents of all pages.
    fn write_pages(&mut self) -> PdfResult<()> {
        for (id, page) in ids(self.offsets.contents).zip(&self.layout.layouts) {
            self.write_page(id, &page)?;
        }
        Ok(())
    }

    /// Write the content of a page.
    fn write_page(&mut self, id: u32, page: &Layout) -> PdfResult<()> {
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
                    if let Some(pos) = next_pos.take() {
                        let x = pos.x.to_pt();
                        let y = (page.dimensions.y - pos.y - Size::pt(active_font.1)).to_pt();
                        text.tm(1.0, 0.0, 0.0, 1.0, x, y);
                    }

                    // Write the text.
                    text.tj(self.fonts[active_font.0].encode_text(&string)?);
                }
                LayoutAction::DebugBox(_, _) => {}
            }
        }

        self.writer.write_obj(id, &text.to_stream())?;

        Ok(())
    }

    /// Write all the fonts.
    fn write_fonts(&mut self) -> PdfResult<()> {
        let mut id = self.offsets.fonts.0;

        for font in &mut self.fonts {
            let name = font
                .read_table::<Name>()?
                .get_decoded(NameEntry::PostScriptName)
                .unwrap_or_else(|| "unknown".to_string());
            let base_font = format!("ABCDEF+{}", name);

            // Write the base font object referencing the CID font.
            self.writer.write_obj(
                id,
                Type0Font::new(
                    base_font.clone(),
                    CMapEncoding::Predefined("Identity-H".to_owned()),
                    id + 1,
                )
                .to_unicode(id + 3),
            )?;

            // Extract information from the head table.
            let head = font.read_table::<Header>()?;

            let font_unit_ratio = 1.0 / (head.units_per_em as f32);
            let font_unit_to_size = |x| Size::pt(font_unit_ratio * x);
            let font_unit_to_glyph_unit = |fu| {
                let size = font_unit_to_size(fu);
                (1000.0 * size.to_pt()).round() as GlyphUnit
            };

            let italic = head.mac_style.contains(MacStyleFlags::ITALIC);
            let bounding_box = Rect::new(
                font_unit_to_glyph_unit(head.x_min as f32),
                font_unit_to_glyph_unit(head.y_min as f32),
                font_unit_to_glyph_unit(head.x_max as f32),
                font_unit_to_glyph_unit(head.y_max as f32),
            );

            // Transform the width into PDF units.
            let widths: Vec<_> = font
                .read_table::<HorizontalMetrics>()?
                .metrics
                .iter()
                .map(|m| font_unit_to_glyph_unit(m.advance_width as f32))
                .collect();

            // Write the CID font referencing the font descriptor.
            let system_info = CIDSystemInfo::new("Adobe", "Identity", 0);
            self.writer.write_obj(
                id + 1,
                CIDFont::new(
                    CIDFontType::Type2,
                    base_font.clone(),
                    system_info.clone(),
                    id + 2,
                )
                .widths(vec![WidthRecord::start(0, widths)]),
            )?;

            // Extract information from the post table.
            let post = font.read_table::<Post>()?;
            let fixed_pitch = post.is_fixed_pitch;
            let italic_angle = post.italic_angle.to_f32();

            // Build the flag set.
            let mut flags = FontFlags::empty();
            flags.set(FontFlags::SERIF, name.contains("Serif"));
            flags.set(FontFlags::FIXED_PITCH, fixed_pitch);
            flags.set(FontFlags::ITALIC, italic);
            flags.insert(FontFlags::SYMBOLIC);
            flags.insert(FontFlags::SMALL_CAP);

            // Extract information from the OS/2 table.
            let os2 = font.read_table::<OS2>()?;

            // Write the font descriptor (contains the global information about the font).
            self.writer.write_obj(
                id + 2,
                FontDescriptor::new(base_font, flags, italic_angle)
                    .font_bbox(bounding_box)
                    .ascent(font_unit_to_glyph_unit(os2.s_typo_ascender as f32))
                    .descent(font_unit_to_glyph_unit(os2.s_typo_descender as f32))
                    .cap_height(font_unit_to_glyph_unit(
                        os2.s_cap_height.unwrap_or(os2.s_typo_ascender) as f32,
                    ))
                    .stem_v((10.0 + 0.244 * (os2.us_weight_class as f32 - 50.0)) as GlyphUnit)
                    .font_file_2(id + 4),
            )?;

            // Write the CMap, which maps glyphs to unicode codepoints.
            let mapping = font
                .read_table::<CharMap>()?
                .mapping
                .iter()
                .map(|(&c, &cid)| (cid, c));
            self.writer
                .write_obj(id + 3, &CMap::new("Custom", system_info, mapping))?;

            // Finally write the subsetted font program.
            self.writer
                .write_obj(id + 4, &FontStream::new(font.data().get_ref()))?;

            id += 5;
        }

        Ok(())
    }
}

/// Create an iterator from a reference pair.
fn ids((start, end): (Ref, Ref)) -> impl Iterator<Item = Ref> {
    start..=end
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

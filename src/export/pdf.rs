//! Exporting into _PDF_ documents.

use std::collections::HashMap;
use std::io::{self, Write};

use fontdock::FaceId;
use tide::content::Content;
use tide::doc::{Catalog, Page, PageTree, Resource, Text};
use tide::font::{
    CIDFont, CIDFontType, CIDSystemInfo, CMap, CMapEncoding, FontDescriptor, FontFlags,
    FontStream, GlyphUnit, Type0Font, WidthRecord,
};
use tide::{PdfWriter, Rect, Ref, Trailer, Version};
use ttf_parser::{name_id, GlyphId};

use crate::font::FontLoader;
use crate::layout::{BoxLayout, LayoutElement};
use crate::length::Length;

/// Export a list of layouts into a _PDF_ document.
///
/// This creates one page per layout. Additionally to the layouts, you need to
/// pass in the font loader used for typesetting such that the fonts can be
/// included in the _PDF_.
///
/// The raw _PDF_ is written into the `target` writable, returning the number of
/// bytes written.
pub fn export<W: Write>(
    layouts: &[BoxLayout],
    loader: &FontLoader,
    target: W,
) -> io::Result<usize> {
    PdfExporter::new(layouts, loader, target)?.write()
}

struct PdfExporter<'a, W: Write> {
    writer: PdfWriter<W>,
    layouts: &'a [BoxLayout],
    loader: &'a FontLoader,
    /// We need to know exactly which indirect reference id will be used for
    /// which objects up-front to correctly declare the document catalogue, page
    /// tree and so on. These offsets are computed in the beginning and stored
    /// here.
    offsets: Offsets,
    // Font remapping, for more information see `remap_fonts`.
    to_pdf: HashMap<FaceId, usize>,
    to_layout: Vec<FaceId>,
}

struct Offsets {
    catalog: Ref,
    page_tree: Ref,
    pages: (Ref, Ref),
    contents: (Ref, Ref),
    fonts: (Ref, Ref),
}

const NUM_OBJECTS_PER_FONT: u32 = 5;

impl<'a, W: Write> PdfExporter<'a, W> {
    fn new(
        layouts: &'a [BoxLayout],
        loader: &'a FontLoader,
        target: W,
    ) -> io::Result<Self> {
        let (to_pdf, to_fontdock) = remap_fonts(layouts);
        let offsets = calculate_offsets(layouts.len(), to_pdf.len());

        Ok(Self {
            writer: PdfWriter::new(target),
            layouts,
            offsets,
            to_pdf,
            to_layout: to_fontdock,
            loader,
        })
    }

    fn write(&mut self) -> io::Result<usize> {
        self.writer.write_header(Version::new(1, 7))?;
        self.write_preface()?;
        self.write_pages()?;
        self.write_fonts()?;
        self.writer.write_xref_table()?;
        self.writer.write_trailer(Trailer::new(self.offsets.catalog))?;
        Ok(self.writer.written())
    }

    fn write_preface(&mut self) -> io::Result<()> {
        // The document catalog.
        self.writer
            .write_obj(self.offsets.catalog, &Catalog::new(self.offsets.page_tree))?;

        // The font resources.
        let start = self.offsets.fonts.0;
        let fonts = (0 .. self.to_pdf.len() as u32)
            .map(|i| Resource::Font(i + 1, start + (NUM_OBJECTS_PER_FONT * i)));

        // The root page tree.
        self.writer.write_obj(
            self.offsets.page_tree,
            PageTree::new().kids(ids(self.offsets.pages)).resources(fonts),
        )?;

        // The page objects (non-root nodes in the page tree).
        for ((page_id, content_id), page) in ids(self.offsets.pages)
            .zip(ids(self.offsets.contents))
            .zip(self.layouts)
        {
            let rect = Rect::new(
                0.0,
                0.0,
                Length::raw(page.size.width).as_pt() as f32,
                Length::raw(page.size.height).as_pt() as f32,
            );

            self.writer.write_obj(
                page_id,
                Page::new(self.offsets.page_tree).media_box(rect).content(content_id),
            )?;
        }

        Ok(())
    }

    fn write_pages(&mut self) -> io::Result<()> {
        for (id, page) in ids(self.offsets.contents).zip(self.layouts) {
            self.write_page(id, &page)?;
        }
        Ok(())
    }

    fn write_page(&mut self, id: u32, page: &BoxLayout) -> io::Result<()> {
        let mut text = Text::new();

        // Font switching actions are only written when the face used for
        // shaped text changes. Hence, we need to remember the active face.
        let mut face = FaceId::MAX;
        let mut size = 0.0;

        for (pos, element) in &page.elements {
            match element {
                LayoutElement::Text(shaped) => {
                    // Check if we need to issue a font switching action.
                    if shaped.face != face || shaped.size != size {
                        face = shaped.face;
                        size = shaped.size;
                        text.tf(
                            self.to_pdf[&shaped.face] as u32 + 1,
                            Length::raw(size).as_pt() as f32,
                        );
                    }

                    let x = Length::raw(pos.x).as_pt();
                    let y = Length::raw(page.size.height - pos.y - size).as_pt();
                    text.tm(1.0, 0.0, 0.0, 1.0, x as f32, y as f32);
                    text.tj(shaped.encode_glyphs_be());
                }
            }
        }

        self.writer.write_obj(id, &text.to_stream())?;

        Ok(())
    }

    fn write_fonts(&mut self) -> io::Result<()> {
        let mut id = self.offsets.fonts.0;

        for &face_id in &self.to_layout {
            let owned_face = self.loader.get_loaded(face_id);
            let face = owned_face.get();

            let name = face
                .names()
                .find(|entry| {
                    entry.name_id() == name_id::POST_SCRIPT_NAME && entry.is_unicode()
                })
                .map(|entry| entry.to_string())
                .flatten()
                .unwrap_or_else(|| "unknown".to_string());

            let base_font = format!("ABCDEF+{}", name);
            let system_info = CIDSystemInfo::new("Adobe", "Identity", 0);

            let units_per_em = face.units_per_em().unwrap_or(1000) as f64;
            let ratio = 1.0 / units_per_em;
            let to_glyph_unit =
                |font_unit: f64| (1000.0 * ratio * font_unit).round() as GlyphUnit;

            let global_bbox = face.global_bounding_box();
            let bbox = Rect::new(
                to_glyph_unit(global_bbox.x_min as f64),
                to_glyph_unit(global_bbox.y_min as f64),
                to_glyph_unit(global_bbox.x_max as f64),
                to_glyph_unit(global_bbox.y_max as f64),
            );

            let monospace = face.is_monospaced();
            let italic = face.is_italic();
            let italic_angle = face.italic_angle().unwrap_or(0.0);
            let ascender = face.typographic_ascender().unwrap_or(0);
            let descender = face.typographic_descender().unwrap_or(0);
            let cap_height = face.capital_height().unwrap_or(ascender);
            let stem_v = 10.0 + 0.244 * (face.weight().to_number() as f32 - 50.0);

            let mut flags = FontFlags::empty();
            flags.set(FontFlags::SERIF, name.contains("Serif"));
            flags.set(FontFlags::FIXED_PITCH, monospace);
            flags.set(FontFlags::ITALIC, italic);
            flags.insert(FontFlags::SYMBOLIC);
            flags.insert(FontFlags::SMALL_CAP);

            let num_glyphs = face.number_of_glyphs();
            let widths: Vec<_> = (0 .. num_glyphs)
                .map(|g| face.glyph_hor_advance(GlyphId(g)).unwrap_or(0))
                .map(|w| to_glyph_unit(w as f64))
                .collect();

            let mut mapping = vec![];
            for subtable in face.character_mapping_subtables() {
                subtable.codepoints(|n| {
                    if let Some(c) = std::char::from_u32(n) {
                        if let Some(g) = face.glyph_index(c) {
                            mapping.push((g.0, c));
                        }
                    }
                })
            }

            // Write the base font object referencing the CID font.
            self.writer.write_obj(
                id,
                Type0Font::new(
                    base_font.clone(),
                    CMapEncoding::Predefined("Identity-H".to_string()),
                    id + 1,
                )
                .to_unicode(id + 3),
            )?;

            // Write the CID font referencing the font descriptor.
            self.writer.write_obj(
                id + 1,
                CIDFont::new(
                    CIDFontType::Type2,
                    base_font.clone(),
                    system_info.clone(),
                    id + 2,
                )
                .widths(vec![WidthRecord::Start(0, widths)]),
            )?;

            // Write the font descriptor (contains metrics about the font).
            self.writer.write_obj(
                id + 2,
                FontDescriptor::new(base_font, flags, italic_angle)
                    .font_bbox(bbox)
                    .ascent(to_glyph_unit(ascender as f64))
                    .descent(to_glyph_unit(descender as f64))
                    .cap_height(to_glyph_unit(cap_height as f64))
                    .stem_v(stem_v as GlyphUnit)
                    .font_file_2(id + 4),
            )?;

            // Write the CMap, which maps glyph ids back to unicode codepoints
            // to enable copying out of the PDF.
            self.writer
                .write_obj(id + 3, &CMap::new("Custom", system_info, mapping))?;

            // Write the face's bytes.
            self.writer.write_obj(id + 4, &FontStream::new(owned_face.data()))?;

            id += NUM_OBJECTS_PER_FONT;
        }

        Ok(())
    }
}

/// Assigns a new PDF-internal index to each used face and returns two mappings:
/// - Forwards from the old face ids to the new pdf indices (hash map)
/// - Backwards from the pdf indices to the old face ids (vec)
fn remap_fonts(layouts: &[BoxLayout]) -> (HashMap<FaceId, usize>, Vec<FaceId>) {
    let mut to_pdf = HashMap::new();
    let mut to_layout = vec![];

    // We want to find out which font faces are used at all. To do that, look at
    // each text element to find out which face is uses.
    for layout in layouts {
        for (_, element) in &layout.elements {
            let LayoutElement::Text(shaped) = element;
            to_pdf.entry(shaped.face).or_insert_with(|| {
                let next_id = to_layout.len();
                to_layout.push(shaped.face);
                next_id
            });
        }
    }

    (to_pdf, to_layout)
}

/// We need to know in advance which ids to use for which objects to
/// cross-reference them. Therefore, we calculate the indices in the beginning.
fn calculate_offsets(layout_count: usize, font_count: usize) -> Offsets {
    let catalog = 1;
    let page_tree = catalog + 1;
    let pages = (page_tree + 1, page_tree + layout_count as Ref);
    let contents = (pages.1 + 1, pages.1 + layout_count as Ref);
    let font_offsets = (contents.1 + 1, contents.1 + 5 * font_count as Ref);

    Offsets {
        catalog,
        page_tree,
        pages,
        contents,
        fonts: font_offsets,
    }
}

fn ids((start, end): (Ref, Ref)) -> impl Iterator<Item = Ref> {
    start ..= end
}

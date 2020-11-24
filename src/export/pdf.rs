//! Exporting into _PDF_ documents.

use std::collections::HashMap;

use fontdock::FaceId;
use pdf_writer::{
    CIDFontType, FontFlags, Name, PdfWriter, Rect, Ref, Str, SystemInfo, TextStream,
};
use ttf_parser::{name_id, GlyphId};

use crate::font::FontLoader;
use crate::geom::Length;
use crate::layout::{BoxLayout, LayoutElement};

/// Export a list of layouts into a _PDF_ document.
///
/// This creates one page per layout. Additionally to the layouts, you need to
/// pass in the font loader used for typesetting such that the fonts can be
/// included in the _PDF_.
///
/// Returns the raw bytes making up the _PDF_ document.
pub fn export(layouts: &[BoxLayout], loader: &FontLoader) -> Vec<u8> {
    PdfExporter::new(layouts, loader).write()
}

struct PdfExporter<'a> {
    writer: PdfWriter,
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
    pages: (i32, i32),
    contents: (i32, i32),
    fonts: (i32, i32),
}

const NUM_OBJECTS_PER_FONT: i32 = 5;

impl<'a> PdfExporter<'a> {
    fn new(layouts: &'a [BoxLayout], loader: &'a FontLoader) -> Self {
        let (to_pdf, to_fontdock) = remap_fonts(layouts);
        let offsets = calculate_offsets(layouts.len(), to_pdf.len());
        let mut writer = PdfWriter::new(1, 7);
        writer.set_indent(2);

        Self {
            writer,
            layouts,
            offsets,
            to_pdf,
            to_layout: to_fontdock,
            loader,
        }
    }

    fn write(mut self) -> Vec<u8> {
        self.write_preface();
        self.write_pages();
        self.write_fonts();
        self.writer.end(self.offsets.catalog)
    }

    fn write_preface(&mut self) {
        // The document catalog.
        self.writer
            .catalog(self.offsets.catalog)
            .pages(self.offsets.page_tree);

        // The root page tree.
        {
            let mut pages = self.writer.pages(self.offsets.page_tree);
            pages.kids(ids(self.offsets.pages));

            let mut resources = pages.resources();
            let mut fonts = resources.fonts();
            for i in 0 .. self.to_pdf.len() {
                let mut buf = itoa::Buffer::new();
                fonts.pair(
                    Name(buf.format(1 + i as i32).as_bytes()),
                    Ref::new(self.offsets.fonts.0 + NUM_OBJECTS_PER_FONT * i as i32),
                );
            }
        }

        // The page objects (non-root nodes in the page tree).
        for ((page_id, content_id), page) in ids(self.offsets.pages)
            .zip(ids(self.offsets.contents))
            .zip(self.layouts)
        {
            let rect = Rect::new(
                0.0,
                0.0,
                page.size.width.to_pt() as f32,
                page.size.height.to_pt() as f32,
            );

            self.writer
                .page(page_id)
                .parent(self.offsets.page_tree)
                .media_box(rect)
                .contents(content_id);
        }
    }

    fn write_pages(&mut self) {
        for (id, page) in ids(self.offsets.contents).zip(self.layouts) {
            self.write_page(id, &page);
        }
    }

    fn write_page(&mut self, id: Ref, page: &BoxLayout) {
        let mut text = TextStream::new();

        // Font switching actions are only written when the face used for
        // shaped text changes. Hence, we need to remember the active face.
        let mut face = FaceId::MAX;
        let mut size = Length::ZERO;

        for (pos, element) in &page.elements {
            match element {
                LayoutElement::Text(shaped) => {
                    // Check if we need to issue a font switching action.
                    if shaped.face != face || shaped.font_size != size {
                        face = shaped.face;
                        size = shaped.font_size;

                        let mut buf = itoa::Buffer::new();
                        text = text.tf(
                            Name(buf.format(1 + self.to_pdf[&shaped.face]).as_bytes()),
                            size.to_pt() as f32,
                        );
                    }

                    let x = pos.x.to_pt();
                    let y = (page.size.height - pos.y - size).to_pt();
                    text = text.tm(1.0, 0.0, 0.0, 1.0, x as f32, y as f32);
                    text = text.tj(&shaped.encode_glyphs_be());
                }

                LayoutElement::Image(_image) => {
                    // TODO: Write image.
                }
            }
        }

        self.writer.stream(id, &text.end());
    }

    fn write_fonts(&mut self) {
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
            let base_font = Name(base_font.as_bytes());
            let system_info = SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            };

            let units_per_em = face.units_per_em().unwrap_or(1000) as f32;
            let ratio = 1.0 / units_per_em;
            let to_glyph_unit = |font_unit: f32| (1000.0 * ratio * font_unit).round();

            let global_bbox = face.global_bounding_box();
            let bbox = Rect::new(
                to_glyph_unit(global_bbox.x_min as f32),
                to_glyph_unit(global_bbox.y_min as f32),
                to_glyph_unit(global_bbox.x_max as f32),
                to_glyph_unit(global_bbox.y_max as f32),
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
            let widths = (0 .. num_glyphs).map(|g| {
                to_glyph_unit(face.glyph_hor_advance(GlyphId(g)).unwrap_or(0) as f32)
            });

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

            let type0_font_id = Ref::new(id);
            let cid_font_id = Ref::new(id + 1);
            let font_descriptor_id = Ref::new(id + 2);
            let cmap_id = Ref::new(id + 3);
            let data_id = Ref::new(id + 4);

            // Write the base font object referencing the CID font.
            self.writer
                .type0_font(type0_font_id)
                .base_font(base_font)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(cid_font_id)
                .to_unicode(cmap_id);

            // Write the CID font referencing the font descriptor.
            self.writer
                .cid_font(cid_font_id, CIDFontType::Type2)
                .base_font(base_font)
                .system_info(system_info)
                .font_descriptor(font_descriptor_id)
                .widths()
                .individual(0, widths);

            // Write the font descriptor (contains metrics about the font).
            self.writer
                .font_descriptor(font_descriptor_id)
                .font_name(base_font)
                .font_flags(flags)
                .font_bbox(bbox)
                .italic_angle(italic_angle)
                .ascent(to_glyph_unit(ascender as f32))
                .descent(to_glyph_unit(descender as f32))
                .cap_height(to_glyph_unit(cap_height as f32))
                .stem_v(stem_v)
                .font_file2(data_id);

            // Write the CMap, which maps glyph ids back to unicode codepoints
            // to enable copying out of the PDF.
            self.writer.cmap(cmap_id, Name(b"Custom"), system_info, mapping);

            // Write the face's bytes.
            self.writer.stream(data_id, owned_face.data());

            id += NUM_OBJECTS_PER_FONT;
        }
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
            if let LayoutElement::Text(shaped) = element {
                to_pdf.entry(shaped.face).or_insert_with(|| {
                    let next_id = to_layout.len();
                    to_layout.push(shaped.face);
                    next_id
                });
            }
        }
    }

    (to_pdf, to_layout)
}

/// We need to know in advance which ids to use for which objects to
/// cross-reference them. Therefore, we calculate the indices in the beginning.
fn calculate_offsets(layout_count: usize, font_count: usize) -> Offsets {
    let catalog = 1;
    let page_tree = catalog + 1;
    let pages = (page_tree + 1, page_tree + layout_count as i32);
    let contents = (pages.1 + 1, pages.1 + layout_count as i32);
    let font_offsets = (contents.1 + 1, contents.1 + 5 * font_count as i32);

    Offsets {
        catalog: Ref::new(catalog),
        page_tree: Ref::new(page_tree),
        pages,
        contents,
        fonts: font_offsets,
    }
}

fn ids((start, end): (i32, i32)) -> impl Iterator<Item = Ref> {
    (start ..= end).map(Ref::new)
}

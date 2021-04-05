//! Exporting into PDF documents.

use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;

use fontdock::FaceId;
use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult, Rgba};
use miniz_oxide::deflate;
use pdf_writer::{
    CidFontType, ColorSpace, Content, Filter, FontFlags, Name, PdfWriter, Rect, Ref, Str,
    SystemInfo, UnicodeCmap,
};
use ttf_parser::{name_id, GlyphId};

use crate::color::Color;
use crate::env::{Env, ImageResource, ResourceId};
use crate::geom::{self, Length, Size};
use crate::layout::{Element, Fill, Frame, Image, Shape};

/// Export a collection of frames into a PDF document.
///
/// This creates one page per frame. In addition to the frames, you need to pass
/// in the environment used for typesetting such that things like fonts and
/// images can be included in the PDF.
///
/// Returns the raw bytes making up the PDF document.
pub fn export(env: &Env, frames: &[Frame]) -> Vec<u8> {
    PdfExporter::new(env, frames).write()
}

struct PdfExporter<'a> {
    writer: PdfWriter,
    frames: &'a [Frame],
    env: &'a Env,
    refs: Refs,
    fonts: Remapper<FaceId>,
    images: Remapper<ResourceId>,
}

impl<'a> PdfExporter<'a> {
    fn new(env: &'a Env, frames: &'a [Frame]) -> Self {
        let mut writer = PdfWriter::new(1, 7);
        writer.set_indent(2);

        let mut fonts = Remapper::new();
        let mut images = Remapper::new();
        let mut alpha_masks = 0;

        for frame in frames {
            for (_, element) in &frame.elements {
                match element {
                    Element::Text(shaped) => fonts.insert(shaped.face_id),
                    Element::Image(image) => {
                        let img = env.resources.loaded::<ImageResource>(image.res);
                        if img.buf.color().has_alpha() {
                            alpha_masks += 1;
                        }
                        images.insert(image.res);
                    }
                    Element::Geometry(_) => {}
                }
            }
        }

        let refs = Refs::new(frames.len(), fonts.len(), images.len(), alpha_masks);

        Self { writer, frames, env, refs, fonts, images }
    }

    fn write(mut self) -> Vec<u8> {
        self.write_structure();
        self.write_pages();
        self.write_fonts();
        self.write_images();
        self.writer.finish(self.refs.catalog)
    }

    fn write_structure(&mut self) {
        // The document catalog.
        self.writer.catalog(self.refs.catalog).pages(self.refs.page_tree);

        // The root page tree.
        let mut pages = self.writer.pages(self.refs.page_tree);
        pages.kids(self.refs.pages());

        let mut resources = pages.resources();
        let mut fonts = resources.fonts();
        for (refs, f) in self.refs.fonts().zip(self.fonts.pdf_indices()) {
            let name = format!("F{}", f);
            fonts.pair(Name(name.as_bytes()), refs.type0_font);
        }

        drop(fonts);

        let mut images = resources.x_objects();
        for (id, im) in self.refs.images().zip(self.images.pdf_indices()) {
            let name = format!("Im{}", im);
            images.pair(Name(name.as_bytes()), id);
        }

        drop(images);
        drop(resources);
        drop(pages);

        // The page objects (non-root nodes in the page tree).
        for ((page_id, content_id), page) in
            self.refs.pages().zip(self.refs.contents()).zip(self.frames)
        {
            self.writer
                .page(page_id)
                .parent(self.refs.page_tree)
                .media_box(Rect::new(
                    0.0,
                    0.0,
                    page.size.width.to_pt() as f32,
                    page.size.height.to_pt() as f32,
                ))
                .contents(content_id);
        }
    }

    fn write_pages(&mut self) {
        for (id, page) in self.refs.contents().zip(self.frames) {
            self.write_page(id, &page);
        }
    }

    fn write_page(&mut self, id: Ref, page: &'a Frame) {
        let mut content = Content::new();

        // We only write font switching actions when the used face changes. To
        // do that, we need to remember the active face.
        let mut face = FaceId::MAX;
        let mut size = Length::ZERO;
        let mut fill: Option<Fill> = None;

        for (pos, element) in &page.elements {
            let x = pos.x.to_pt() as f32;
            let y = (page.size.height - pos.y).to_pt() as f32;

            match element {
                &Element::Image(Image { res, size: Size { width, height } }) => {
                    let name = format!("Im{}", self.images.map(res));
                    let w = width.to_pt() as f32;
                    let h = height.to_pt() as f32;

                    content.save_state();
                    content.matrix(w, 0.0, 0.0, h, x, y - h);
                    content.x_object(Name(name.as_bytes()));
                    content.restore_state();
                }

                Element::Geometry(geometry) => {
                    content.save_state();
                    write_fill(&mut content, geometry.fill);

                    match geometry.shape {
                        Shape::Rect(Size { width, height }) => {
                            let w = width.to_pt() as f32;
                            let h = height.to_pt() as f32;
                            if w > 0.0 && h > 0.0 {
                                content.rect(x, y - h, w, h, false, true);
                            }
                        }

                        Shape::Ellipse(size) => {
                            let path = geom::ellipse_path(size);
                            write_path(&mut content, x, y, &path, false, true);
                        }

                        Shape::Path(ref path) => {
                            write_path(&mut content, x, y, path, false, true)
                        }
                    }

                    content.restore_state();
                }

                Element::Text(shaped) => {
                    if fill != Some(shaped.color) {
                        write_fill(&mut content, shaped.color);
                        fill = Some(shaped.color);
                    }

                    let mut text = content.text();

                    // Then, also check if we need to issue a font switching
                    // action.
                    if shaped.face_id != face || shaped.size != size {
                        face = shaped.face_id;
                        size = shaped.size;

                        let name = format!("F{}", self.fonts.map(shaped.face_id));
                        text.font(Name(name.as_bytes()), size.to_pt() as f32);
                    }

                    // TODO: Respect individual glyph offsets.
                    text.matrix(1.0, 0.0, 0.0, 1.0, x, y);
                    text.show(&shaped.encode_glyphs_be());
                }
            }
        }

        self.writer.stream(id, &content.finish());
    }

    fn write_fonts(&mut self) {
        for (refs, face_id) in self.refs.fonts().zip(self.fonts.layout_indices()) {
            let face = self.env.fonts.face(face_id);
            let ttf = face.ttf();

            let name = ttf
                .names()
                .find(|entry| {
                    entry.name_id() == name_id::POST_SCRIPT_NAME && entry.is_unicode()
                })
                .and_then(|entry| entry.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let base_font = format!("ABCDEF+{}", name);
            let base_font = Name(base_font.as_bytes());
            let cmap_name = Name(b"Custom");
            let system_info = SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            };

            let mut flags = FontFlags::empty();
            flags.set(FontFlags::SERIF, name.contains("Serif"));
            flags.set(FontFlags::FIXED_PITCH, ttf.is_monospaced());
            flags.set(FontFlags::ITALIC, ttf.is_italic());
            flags.insert(FontFlags::SYMBOLIC);
            flags.insert(FontFlags::SMALL_CAP);

            // Convert from OpenType font units to PDF glyph units.
            let em_per_unit = 1.0 / ttf.units_per_em().unwrap_or(1000) as f32;
            let convert = |font_unit: f32| (1000.0 * em_per_unit * font_unit).round();
            let convert_i16 = |font_unit: i16| convert(font_unit as f32);
            let convert_u16 = |font_unit: u16| convert(font_unit as f32);

            let global_bbox = ttf.global_bounding_box();
            let bbox = Rect::new(
                convert_i16(global_bbox.x_min),
                convert_i16(global_bbox.y_min),
                convert_i16(global_bbox.x_max),
                convert_i16(global_bbox.y_max),
            );

            let italic_angle = ttf.italic_angle().unwrap_or(0.0);
            let ascender = convert_i16(ttf.typographic_ascender().unwrap_or(0));
            let descender = convert_i16(ttf.typographic_descender().unwrap_or(0));
            let cap_height = ttf.capital_height().map(convert_i16);
            let stem_v = 10.0 + 0.244 * (f32::from(ttf.weight().to_number()) - 50.0);

            // Write the base font object referencing the CID font.
            self.writer
                .type0_font(refs.type0_font)
                .base_font(base_font)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(refs.cid_font)
                .to_unicode(refs.cmap);

            // Write the CID font referencing the font descriptor.
            self.writer
                .cid_font(refs.cid_font, CidFontType::Type2)
                .base_font(base_font)
                .system_info(system_info)
                .font_descriptor(refs.font_descriptor)
                .widths()
                .individual(0, {
                    let num_glyphs = ttf.number_of_glyphs();
                    (0 .. num_glyphs).map(|g| {
                        let advance = ttf.glyph_hor_advance(GlyphId(g));
                        convert_u16(advance.unwrap_or(0))
                    })
                });

            // Write the font descriptor (contains metrics about the font).
            self.writer
                .font_descriptor(refs.font_descriptor)
                .font_name(base_font)
                .font_flags(flags)
                .font_bbox(bbox)
                .italic_angle(italic_angle)
                .ascent(ascender)
                .descent(descender)
                .cap_height(cap_height.unwrap_or(ascender))
                .stem_v(stem_v)
                .font_file2(refs.data);

            // Write the to-unicode character map, which maps glyph ids back to
            // unicode codepoints to enable copying out of the PDF.
            self.writer
                .cmap(refs.cmap, &{
                    let mut cmap = UnicodeCmap::new(cmap_name, system_info);
                    for subtable in ttf.character_mapping_subtables() {
                        subtable.codepoints(|n| {
                            if let Some(c) = std::char::from_u32(n) {
                                if let Some(g) = ttf.glyph_index(c) {
                                    cmap.pair(g.0, c);
                                }
                            }
                        })
                    }
                    cmap.finish()
                })
                .name(cmap_name)
                .system_info(system_info);

            // Write the face's bytes.
            self.writer.stream(refs.data, face.data());
        }
    }

    fn write_images(&mut self) {
        let mut masks_seen = 0;

        for (id, resource) in self.refs.images().zip(self.images.layout_indices()) {
            let img = self.env.resources.loaded::<ImageResource>(resource);
            let (width, height) = img.buf.dimensions();

            // Add the primary image.
            if let Ok((data, filter, color_space)) = encode_image(img) {
                let mut image = self.writer.image(id, &data);
                image.inner().filter(filter);
                image.width(width as i32);
                image.height(height as i32);
                image.color_space(color_space);
                image.bits_per_component(8);

                // Add a second gray-scale image containing the alpha values if
                // this image has an alpha channel.
                if img.buf.color().has_alpha() {
                    let (alpha_data, alpha_filter) = encode_alpha(img);
                    let mask_id = self.refs.alpha_mask(masks_seen);
                    image.s_mask(mask_id);
                    drop(image);

                    let mut mask = self.writer.image(mask_id, &alpha_data);
                    mask.inner().filter(alpha_filter);
                    mask.width(width as i32);
                    mask.height(height as i32);
                    mask.color_space(ColorSpace::DeviceGray);
                    mask.bits_per_component(8);

                    masks_seen += 1;
                }
            } else {
                // TODO: Warn that image could not be encoded.
                self.writer
                    .image(id, &[])
                    .width(0)
                    .height(0)
                    .color_space(ColorSpace::DeviceGray)
                    .bits_per_component(1);
            }
        }
    }
}

/// Write a fill change into a content stream.
fn write_fill(content: &mut Content, fill: Fill) {
    match fill {
        Fill::Color(Color::Rgba(c)) => {
            content.fill_rgb(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
        }
        Fill::Image(_) => todo!(),
    }
}

/// Write a path into a content stream.
fn write_path(
    content: &mut Content,
    x: f32,
    y: f32,
    path: &geom::Path,
    stroke: bool,
    fill: bool,
) {
    let f = |length: Length| length.to_pt() as f32;
    let mut builder = content.path(stroke, fill);
    for elem in &path.0 {
        match elem {
            geom::PathElement::MoveTo(p) => builder.move_to(x + f(p.x), y + f(p.y)),
            geom::PathElement::LineTo(p) => builder.line_to(x + f(p.x), y + f(p.y)),
            geom::PathElement::CubicTo(p1, p2, p3) => builder.cubic_to(
                x + f(p1.x),
                y + f(p1.y),
                x + f(p2.x),
                y + f(p2.y),
                x + f(p3.x),
                y + f(p3.y),
            ),
            geom::PathElement::ClosePath => builder.close_path(),
        };
    }
}

/// The compression level for the deflating.
const DEFLATE_LEVEL: u8 = 6;

/// Encode an image with a suitable filter.
///
/// Skips the alpha channel as that's encoded separately.
fn encode_image(img: &ImageResource) -> ImageResult<(Vec<u8>, Filter, ColorSpace)> {
    let mut data = vec![];
    let (filter, space) = match (img.format, &img.buf) {
        // 8-bit gray JPEG.
        (ImageFormat::Jpeg, DynamicImage::ImageLuma8(_)) => {
            img.buf.write_to(&mut data, img.format)?;
            (Filter::DctDecode, ColorSpace::DeviceGray)
        }

        // 8-bit Rgb JPEG (Cmyk JPEGs get converted to Rgb earlier).
        (ImageFormat::Jpeg, DynamicImage::ImageRgb8(_)) => {
            img.buf.write_to(&mut data, img.format)?;
            (Filter::DctDecode, ColorSpace::DeviceRgb)
        }

        // TODO: Encode flate streams with PNG-predictor?

        // 8-bit gray PNG.
        (ImageFormat::Png, DynamicImage::ImageLuma8(luma)) => {
            data = deflate::compress_to_vec_zlib(&luma.as_raw(), DEFLATE_LEVEL);
            (Filter::FlateDecode, ColorSpace::DeviceGray)
        }

        // Anything else (including Rgb(a) PNGs).
        (_, buf) => {
            let (width, height) = buf.dimensions();
            let mut pixels = Vec::with_capacity(3 * width as usize * height as usize);
            for (_, _, Rgba([r, g, b, _])) in buf.pixels() {
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
            }

            data = deflate::compress_to_vec_zlib(&pixels, DEFLATE_LEVEL);
            (Filter::FlateDecode, ColorSpace::DeviceRgb)
        }
    };
    Ok((data, filter, space))
}

/// Encode an image's alpha channel if present.
fn encode_alpha(img: &ImageResource) -> (Vec<u8>, Filter) {
    let pixels: Vec<_> = img.buf.pixels().map(|(_, _, Rgba([_, _, _, a]))| a).collect();
    let data = deflate::compress_to_vec_zlib(&pixels, DEFLATE_LEVEL);
    (data, Filter::FlateDecode)
}

/// We need to know exactly which indirect reference id will be used for which
/// objects up-front to correctly declare the document catalogue, page tree and
/// so on. These offsets are computed in the beginning and stored here.
struct Refs {
    catalog: Ref,
    page_tree: Ref,
    pages_start: i32,
    contents_start: i32,
    fonts_start: i32,
    images_start: i32,
    alpha_masks_start: i32,
    end: i32,
}

struct FontRefs {
    type0_font: Ref,
    cid_font: Ref,
    font_descriptor: Ref,
    cmap: Ref,
    data: Ref,
}

impl Refs {
    const OBJECTS_PER_FONT: usize = 5;

    fn new(frames: usize, fonts: usize, images: usize, alpha_masks: usize) -> Self {
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages_start = page_tree + 1;
        let contents_start = pages_start + frames as i32;
        let fonts_start = contents_start + frames as i32;
        let images_start = fonts_start + (Self::OBJECTS_PER_FONT * fonts) as i32;
        let alpha_masks_start = images_start + images as i32;
        let end = alpha_masks_start + alpha_masks as i32;

        Self {
            catalog: Ref::new(catalog),
            page_tree: Ref::new(page_tree),
            pages_start,
            contents_start,
            fonts_start,
            images_start,
            alpha_masks_start,
            end,
        }
    }

    fn pages(&self) -> impl Iterator<Item = Ref> {
        (self.pages_start .. self.contents_start).map(Ref::new)
    }

    fn contents(&self) -> impl Iterator<Item = Ref> {
        (self.contents_start .. self.images_start).map(Ref::new)
    }

    fn fonts(&self) -> impl Iterator<Item = FontRefs> {
        (self.fonts_start .. self.images_start)
            .step_by(Self::OBJECTS_PER_FONT)
            .map(|id| FontRefs {
                type0_font: Ref::new(id),
                cid_font: Ref::new(id + 1),
                font_descriptor: Ref::new(id + 2),
                cmap: Ref::new(id + 3),
                data: Ref::new(id + 4),
            })
    }

    fn images(&self) -> impl Iterator<Item = Ref> {
        (self.images_start .. self.end).map(Ref::new)
    }

    fn alpha_mask(&self, i: usize) -> Ref {
        Ref::new(self.alpha_masks_start + i as i32)
    }
}

/// Used to assign new, consecutive PDF-internal indices to things.
struct Remapper<Index> {
    /// Forwards from the old indices to the new pdf indices.
    to_pdf: HashMap<Index, usize>,
    /// Backwards from the pdf indices to the old indices.
    to_layout: Vec<Index>,
}

impl<Index> Remapper<Index>
where
    Index: Copy + Eq + Hash,
{
    fn new() -> Self {
        Self {
            to_pdf: HashMap::new(),
            to_layout: vec![],
        }
    }

    fn len(&self) -> usize {
        self.to_layout.len()
    }

    fn insert(&mut self, index: Index) {
        let to_layout = &mut self.to_layout;
        self.to_pdf.entry(index).or_insert_with(|| {
            let pdf_index = to_layout.len();
            to_layout.push(index);
            pdf_index
        });
    }

    fn map(&self, index: Index) -> usize {
        self.to_pdf[&index]
    }

    fn pdf_indices(&self) -> impl Iterator<Item = usize> {
        0 .. self.to_pdf.len()
    }

    fn layout_indices(&self) -> impl Iterator<Item = Index> + '_ {
        self.to_layout.iter().copied()
    }
}

//! Exporting into PDF documents.

use std::cmp::Eq;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::rc::Rc;

use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult, Rgba};
use miniz_oxide::deflate;
use pdf_writer::{
    ActionType, AnnotationType, CidFontType, ColorSpace, Content, Filter, FontFlags,
    Name, PdfWriter, Rect, Ref, Str, SystemInfo, UnicodeCmap,
};
use ttf_parser::{name_id, GlyphId, Tag};

use super::subset;
use crate::color::Color;
use crate::font::{find_name, FaceId, FontStore};
use crate::geom::{self, Em, Length, Size};
use crate::image::{Image, ImageId, ImageStore};
use crate::layout::{Element, Frame, Geometry, Paint};
use crate::Context;

/// Export a collection of frames into a PDF document.
///
/// This creates one page per frame. In addition to the frames, you need to pass
/// in the context used during compilation such that things like fonts and
/// images can be included in the PDF.
///
/// Returns the raw bytes making up the PDF document.
pub fn pdf(ctx: &Context, frames: &[Rc<Frame>]) -> Vec<u8> {
    PdfExporter::new(ctx, frames).write()
}

struct PdfExporter<'a> {
    writer: PdfWriter,
    refs: Refs,
    frames: &'a [Rc<Frame>],
    fonts: &'a FontStore,
    images: &'a ImageStore,
    glyphs: HashMap<FaceId, HashSet<u16>>,
    font_map: Remapper<FaceId>,
    image_map: Remapper<ImageId>,
}

impl<'a> PdfExporter<'a> {
    fn new(ctx: &'a Context, frames: &'a [Rc<Frame>]) -> Self {
        let mut glyphs = HashMap::<FaceId, HashSet<u16>>::new();
        let mut font_map = Remapper::new();
        let mut image_map = Remapper::new();
        let mut alpha_masks = 0;

        for frame in frames {
            for (_, element) in frame.elements() {
                match *element {
                    Element::Text(ref text) => {
                        font_map.insert(text.face_id);
                        let set = glyphs.entry(text.face_id).or_default();
                        set.extend(text.glyphs.iter().map(|g| g.id));
                    }
                    Element::Geometry(_, _) => {}
                    Element::Image(id, _) => {
                        let img = ctx.images.get(id);
                        if img.buf.color().has_alpha() {
                            alpha_masks += 1;
                        }
                        image_map.insert(id);
                    }
                    Element::Link(_, _) => {}
                }
            }
        }

        let mut writer = PdfWriter::new(1, 7);
        writer.set_indent(2);

        Self {
            writer,
            refs: Refs::new(frames.len(), font_map.len(), image_map.len(), alpha_masks),
            frames,
            fonts: &ctx.fonts,
            images: &ctx.images,
            glyphs,
            font_map,
            image_map,
        }
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
        for (refs, f) in self.refs.fonts().zip(self.font_map.pdf_indices()) {
            let name = format!("F{}", f);
            fonts.pair(Name(name.as_bytes()), refs.type0_font);
        }

        drop(fonts);

        let mut images = resources.x_objects();
        for (id, im) in self.refs.images().zip(self.image_map.pdf_indices()) {
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
            let w = page.size.w.to_pt() as f32;
            let h = page.size.h.to_pt() as f32;

            let mut page_writer = self.writer.page(page_id);
            page_writer
                .parent(self.refs.page_tree)
                .media_box(Rect::new(0.0, 0.0, w, h));

            let mut annotations = page_writer.annotations();
            for (pos, element) in page.elements() {
                if let Element::Link(href, size) = element {
                    let x = pos.x.to_pt() as f32;
                    let y = (page.size.h - pos.y).to_pt() as f32;
                    let w = size.w.to_pt() as f32;
                    let h = size.h.to_pt() as f32;

                    annotations
                        .push()
                        .subtype(AnnotationType::Link)
                        .rect(Rect::new(x, y - h, x + w, y))
                        .action()
                        .action_type(ActionType::Uri)
                        .uri(Str(href.as_bytes()));
                }
            }

            drop(annotations);
            page_writer.contents(content_id);
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
        let mut face_id = None;
        let mut size = Length::zero();
        let mut fill: Option<Paint> = None;

        for (pos, element) in page.elements() {
            let x = pos.x.to_pt() as f32;
            let y = (page.size.h - pos.y).to_pt() as f32;

            match *element {
                Element::Text(ref text) => {
                    if fill != Some(text.fill) {
                        write_fill(&mut content, text.fill);
                        fill = Some(text.fill);
                    }

                    let mut text_writer = content.text();

                    // Then, also check if we need to issue a font switching
                    // action.
                    if face_id != Some(text.face_id) || text.size != size {
                        face_id = Some(text.face_id);
                        size = text.size;

                        let name = format!("F{}", self.font_map.map(text.face_id));
                        text_writer.font(Name(name.as_bytes()), size.to_pt() as f32);
                    }

                    let face = self.fonts.get(text.face_id);

                    // Position the text.
                    text_writer.matrix(1.0, 0.0, 0.0, 1.0, x, y);

                    let mut positioned = text_writer.show_positioned();
                    let mut adjustment = Em::zero();
                    let mut encoded = vec![];

                    // Write the glyphs with kerning adjustments.
                    for glyph in &text.glyphs {
                        adjustment += glyph.x_offset;

                        if !adjustment.is_zero() {
                            if !encoded.is_empty() {
                                positioned.show(Str(&encoded));
                                encoded.clear();
                            }

                            positioned.adjust(-adjustment.to_pdf());
                            adjustment = Em::zero();
                        }

                        encoded.push((glyph.id >> 8) as u8);
                        encoded.push((glyph.id & 0xff) as u8);

                        if let Some(advance) = face.advance(glyph.id) {
                            adjustment += glyph.x_advance - advance;
                        }

                        adjustment -= glyph.x_offset;
                    }

                    if !encoded.is_empty() {
                        positioned.show(Str(&encoded));
                    }
                }

                Element::Geometry(ref geometry, paint) => {
                    content.save_state();

                    match *geometry {
                        Geometry::Rect(Size { w, h }) => {
                            let w = w.to_pt() as f32;
                            let h = h.to_pt() as f32;
                            if w > 0.0 && h > 0.0 {
                                write_fill(&mut content, paint);
                                content.rect(x, y - h, w, h, false, true);
                            }
                        }
                        Geometry::Ellipse(size) => {
                            let path = geom::Path::ellipse(size);
                            write_fill(&mut content, paint);
                            write_path(&mut content, x, y, &path, false, true);
                        }
                        Geometry::Line(target, thickness) => {
                            write_stroke(&mut content, paint, thickness.to_pt() as f32);
                            content.path(true, false).move_to(x, y).line_to(
                                x + target.x.to_pt() as f32,
                                y - target.y.to_pt() as f32,
                            );
                        }
                        Geometry::Path(ref path) => {
                            write_fill(&mut content, paint);
                            write_path(&mut content, x, y, path, false, true)
                        }
                    }

                    content.restore_state();
                }

                Element::Image(id, Size { w, h }) => {
                    let name = format!("Im{}", self.image_map.map(id));
                    let w = w.to_pt() as f32;
                    let h = h.to_pt() as f32;

                    content.save_state();
                    content.matrix(w, 0.0, 0.0, h, x, y - h);
                    content.x_object(Name(name.as_bytes()));
                    content.restore_state();
                }

                Element::Link(_, _) => {}
            }
        }

        self.writer.stream(id, &content.finish());
    }

    fn write_fonts(&mut self) {
        for (refs, face_id) in self.refs.fonts().zip(self.font_map.layout_indices()) {
            let glyphs = &self.glyphs[&face_id];
            let face = self.fonts.get(face_id);
            let ttf = face.ttf();

            let name = find_name(ttf.names(), name_id::POST_SCRIPT_NAME)
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

            let global_bbox = ttf.global_bounding_box();
            let bbox = Rect::new(
                face.to_em(global_bbox.x_min).to_pdf(),
                face.to_em(global_bbox.y_min).to_pdf(),
                face.to_em(global_bbox.x_max).to_pdf(),
                face.to_em(global_bbox.y_max).to_pdf(),
            );

            let italic_angle = ttf.italic_angle().unwrap_or(0.0);
            let ascender = face.ascender.to_pdf();
            let descender = face.descender.to_pdf();
            let cap_height = face.cap_height.to_pdf();
            let stem_v = 10.0 + 0.244 * (f32::from(ttf.weight().to_number()) - 50.0);

            // Check for the presence of CFF outlines to select the correct
            // CID-Font subtype.
            let subtype = match ttf
                .table_data(Tag::from_bytes(b"CFF "))
                .or(ttf.table_data(Tag::from_bytes(b"CFF2")))
            {
                Some(_) => CidFontType::Type0,
                None => CidFontType::Type2,
            };

            // Write the base font object referencing the CID font.
            self.writer
                .type0_font(refs.type0_font)
                .base_font(base_font)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(refs.cid_font)
                .to_unicode(refs.cmap);

            // Write the CID font referencing the font descriptor.
            self.writer
                .cid_font(refs.cid_font, subtype)
                .base_font(base_font)
                .system_info(system_info)
                .font_descriptor(refs.font_descriptor)
                .cid_to_gid_map_predefined(Name(b"Identity"))
                .widths()
                .individual(0, {
                    let num_glyphs = ttf.number_of_glyphs();
                    (0 .. num_glyphs).map(|g| {
                        let x = ttf.glyph_hor_advance(GlyphId(g)).unwrap_or(0);
                        face.to_em(x).to_pdf()
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
                .cap_height(cap_height)
                .stem_v(stem_v)
                .font_file2(refs.data);

            // Write the to-unicode character map, which maps glyph ids back to
            // unicode codepoints to enable copying out of the PDF.
            self.writer.cmap(refs.cmap, &{
                // Deduplicate glyph-to-unicode mappings with a set.
                let mut mapping = BTreeMap::new();
                for subtable in ttf.character_mapping_subtables() {
                    if subtable.is_unicode() {
                        subtable.codepoints(|n| {
                            if let Some(c) = std::char::from_u32(n) {
                                if let Some(GlyphId(g)) = ttf.glyph_index(c) {
                                    if glyphs.contains(&g) {
                                        mapping.insert(g, c);
                                    }
                                }
                            }
                        });
                    }
                }

                let mut cmap = UnicodeCmap::new(cmap_name, system_info);
                for (g, c) in mapping {
                    cmap.pair(g, c);
                }
                cmap.finish()
            });

            // Subset and write the face's bytes.
            let original = face.buffer();
            let subsetted = subset(original, face.index(), glyphs.iter().copied());
            let data = subsetted.as_deref().unwrap_or(original);
            self.writer.stream(refs.data, data);
        }
    }

    fn write_images(&mut self) {
        let mut masks_seen = 0;

        for (id, image_id) in self.refs.images().zip(self.image_map.layout_indices()) {
            let img = self.images.get(image_id);
            let (width, height) = img.buf.dimensions();

            // Add the primary image.
            if let Ok((data, filter, color_space)) = encode_image(img) {
                let mut image = self.writer.image(id, &data);
                image.filter(filter);
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
                    mask.filter(alpha_filter);
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
fn write_fill(content: &mut Content, fill: Paint) {
    let Paint::Color(Color::Rgba(c)) = fill;
    content.fill_rgb(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0);
}

/// Write a stroke change into a content stream.
fn write_stroke(content: &mut Content, stroke: Paint, thickness: f32) {
    match stroke {
        Paint::Color(Color::Rgba(c)) => {
            content.stroke_rgb(
                c.r as f32 / 255.0,
                c.g as f32 / 255.0,
                c.b as f32 / 255.0,
            );
        }
    }
    content.line_width(thickness);
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
fn encode_image(img: &Image) -> ImageResult<(Vec<u8>, Filter, ColorSpace)> {
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
fn encode_alpha(img: &Image) -> (Vec<u8>, Filter) {
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

    fn new(pages: usize, fonts: usize, images: usize, alpha_masks: usize) -> Self {
        let catalog = 1;
        let page_tree = catalog + 1;
        let pages_start = page_tree + 1;
        let contents_start = pages_start + pages as i32;
        let fonts_start = contents_start + pages as i32;
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

/// Additional methods for [`Em`].
trait EmExt {
    /// Convert an em length to a number of PDF font units.
    fn to_pdf(self) -> f32;
}

impl EmExt for Em {
    fn to_pdf(self) -> f32 {
        1000.0 * self.get() as f32
    }
}

//! Exporting into PDF documents.

use std::cmp::Eq;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;

use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult, Rgba};
use pdf_writer::types::{
    ActionType, AnnotationType, CidFontType, ColorSpaceOperand, FontFlags, SystemInfo,
    UnicodeCmap,
};
use pdf_writer::writers::ColorSpace;
use pdf_writer::{Content, Filter, Finish, Name, PdfWriter, Rect, Ref, Str, TextStr};
use ttf_parser::{name_id, GlyphId, Tag};

use super::subset::subset;
use crate::font::{find_name, FaceId, FontStore};
use crate::frame::{Element, Frame, Geometry, Group, Shape, Stroke, Text};
use crate::geom::{self, Color, Em, Length, Numeric, Paint, Point, Size, Transform};
use crate::image::{Image, ImageId, ImageStore, RasterImage};
use crate::Context;

/// Export a collection of frames into a PDF file.
///
/// This creates one page per frame. In addition to the frames, you need to pass
/// in the context used during compilation so that fonts and images can be
/// included in the PDF.
///
/// Returns the raw bytes making up the PDF file.
pub fn pdf(ctx: &Context, frames: &[Arc<Frame>]) -> Vec<u8> {
    PdfExporter::new(ctx).export(frames)
}

/// Identifies the sRGB color space definition.
const SRGB: Name<'static> = Name(b"sRGB");

/// An exporter for a whole PDF document.
struct PdfExporter<'a> {
    fonts: &'a FontStore,
    images: &'a ImageStore,
    writer: PdfWriter,
    alloc: Ref,
    pages: Vec<Page>,
    face_map: Remapper<FaceId>,
    face_refs: Vec<Ref>,
    glyph_sets: HashMap<FaceId, HashSet<u16>>,
    image_map: Remapper<ImageId>,
    image_refs: Vec<Ref>,
}

impl<'a> PdfExporter<'a> {
    fn new(ctx: &'a Context) -> Self {
        Self {
            fonts: &ctx.fonts,
            images: &ctx.images,
            writer: PdfWriter::new(),
            alloc: Ref::new(1),
            pages: vec![],
            face_map: Remapper::new(),
            face_refs: vec![],
            glyph_sets: HashMap::new(),
            image_map: Remapper::new(),
            image_refs: vec![],
        }
    }

    fn export(mut self, frames: &[Arc<Frame>]) -> Vec<u8> {
        self.build_pages(frames);
        self.write_fonts();
        self.write_images();
        self.write_structure()
    }

    fn build_pages(&mut self, frames: &[Arc<Frame>]) {
        for frame in frames {
            let page = PageExporter::new(self).export(frame);
            self.pages.push(page);
        }
    }

    fn write_fonts(&mut self) {
        for face_id in self.face_map.layout_indices() {
            let type0_ref = self.alloc.bump();
            let cid_ref = self.alloc.bump();
            let descriptor_ref = self.alloc.bump();
            let cmap_ref = self.alloc.bump();
            let data_ref = self.alloc.bump();
            self.face_refs.push(type0_ref);

            let glyphs = &self.glyph_sets[&face_id];
            let face = self.fonts.get(face_id);
            let metrics = face.metrics();
            let ttf = face.ttf();

            let postscript_name = find_name(ttf, name_id::POST_SCRIPT_NAME)
                .unwrap_or_else(|| "unknown".to_string());

            let base_font = format_eco!("ABCDEF+{}", postscript_name);
            let base_font = Name(base_font.as_bytes());
            let cmap_name = Name(b"Custom");
            let system_info = SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            };

            // Write the base font object referencing the CID font.
            self.writer
                .type0_font(type0_ref)
                .base_font(base_font)
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(cid_ref)
                .to_unicode(cmap_ref);

            // Check for the presence of CFF outlines to select the correct
            // CID-Font subtype.
            let subtype = match ttf
                .table_data(Tag::from_bytes(b"CFF "))
                .or(ttf.table_data(Tag::from_bytes(b"CFF2")))
            {
                Some(_) => CidFontType::Type0,
                None => CidFontType::Type2,
            };

            // Write the CID font referencing the font descriptor.
            self.writer
                .cid_font(cid_ref)
                .subtype(subtype)
                .base_font(base_font)
                .system_info(system_info)
                .font_descriptor(descriptor_ref)
                .cid_to_gid_map_predefined(Name(b"Identity"))
                .widths()
                .consecutive(0, {
                    let num_glyphs = ttf.number_of_glyphs();
                    (0 .. num_glyphs).map(|g| {
                        let x = ttf.glyph_hor_advance(GlyphId(g)).unwrap_or(0);
                        face.to_em(x).to_font_units()
                    })
                });

            let mut flags = FontFlags::empty();
            flags.set(FontFlags::SERIF, postscript_name.contains("Serif"));
            flags.set(FontFlags::FIXED_PITCH, ttf.is_monospaced());
            flags.set(FontFlags::ITALIC, ttf.is_italic());
            flags.insert(FontFlags::SYMBOLIC);
            flags.insert(FontFlags::SMALL_CAP);

            let global_bbox = ttf.global_bounding_box();
            let bbox = Rect::new(
                face.to_em(global_bbox.x_min).to_font_units(),
                face.to_em(global_bbox.y_min).to_font_units(),
                face.to_em(global_bbox.x_max).to_font_units(),
                face.to_em(global_bbox.y_max).to_font_units(),
            );

            let italic_angle = ttf.italic_angle().unwrap_or(0.0);
            let ascender = metrics.ascender.to_font_units();
            let descender = metrics.descender.to_font_units();
            let cap_height = metrics.cap_height.to_font_units();
            let stem_v = 10.0 + 0.244 * (f32::from(ttf.weight().to_number()) - 50.0);

            // Write the font descriptor (contains metrics about the font).
            self.writer
                .font_descriptor(descriptor_ref)
                .name(base_font)
                .flags(flags)
                .bbox(bbox)
                .italic_angle(italic_angle)
                .ascent(ascender)
                .descent(descender)
                .cap_height(cap_height)
                .stem_v(stem_v)
                .font_file2(data_ref);

            // Compute a reverse mapping from glyphs to unicode.
            let cmap = {
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
                cmap
            };

            // Write the /ToUnicode character map, which maps glyph ids back to
            // unicode codepoints to enable copying out of the PDF.
            self.writer
                .cmap(cmap_ref, &deflate(&cmap.finish()))
                .filter(Filter::FlateDecode);

            // Subset and write the face's bytes.
            let buffer = face.buffer();
            let subsetted = subset(buffer, face.index(), glyphs);
            let data = subsetted.as_deref().unwrap_or(buffer);
            self.writer
                .stream(data_ref, &deflate(data))
                .filter(Filter::FlateDecode);
        }
    }

    fn write_images(&mut self) {
        for image_id in self.image_map.layout_indices() {
            let image_ref = self.alloc.bump();
            self.image_refs.push(image_ref);

            let img = self.images.get(image_id);
            let width = img.width();
            let height = img.height();

            // Add the primary image.
            match img {
                Image::Raster(img) => {
                    if let Ok((data, filter, has_color)) = encode_image(img) {
                        let mut image = self.writer.image_xobject(image_ref, &data);
                        image.filter(filter);
                        image.width(width as i32);
                        image.height(height as i32);
                        image.bits_per_component(8);

                        let space = image.color_space();
                        if has_color {
                            space.device_rgb();
                        } else {
                            space.device_gray();
                        }

                        // Add a second gray-scale image containing the alpha values if
                        // this image has an alpha channel.
                        if img.buf.color().has_alpha() {
                            let (alpha_data, alpha_filter) = encode_alpha(img);
                            let mask_ref = self.alloc.bump();
                            image.s_mask(mask_ref);
                            image.finish();

                            let mut mask =
                                self.writer.image_xobject(mask_ref, &alpha_data);
                            mask.filter(alpha_filter);
                            mask.width(width as i32);
                            mask.height(height as i32);
                            mask.color_space().device_gray();
                            mask.bits_per_component(8);
                        }
                    } else {
                        // TODO: Warn that image could not be encoded.
                        self.writer
                            .image_xobject(image_ref, &[])
                            .width(0)
                            .height(0)
                            .bits_per_component(1)
                            .color_space()
                            .device_gray();
                    }
                }
                Image::Svg(img) => {
                    let next_ref = svg2pdf::convert_tree_into(
                        &img.0,
                        svg2pdf::Options::default(),
                        &mut self.writer,
                        image_ref,
                    );
                    self.alloc = next_ref;
                }
            }
        }
    }

    fn write_structure(mut self) -> Vec<u8> {
        // The root page tree.
        let page_tree_ref = self.alloc.bump();

        // The page objects (non-root nodes in the page tree).
        let mut page_refs = vec![];
        for page in self.pages {
            let page_id = self.alloc.bump();
            let content_id = self.alloc.bump();
            page_refs.push(page_id);

            let mut page_writer = self.writer.page(page_id);
            page_writer.parent(page_tree_ref);

            let w = page.size.x.to_f32();
            let h = page.size.y.to_f32();
            page_writer.media_box(Rect::new(0.0, 0.0, w, h));
            page_writer.contents(content_id);

            let mut annotations = page_writer.annotations();
            for (url, rect) in page.links {
                annotations
                    .push()
                    .subtype(AnnotationType::Link)
                    .rect(rect)
                    .action()
                    .action_type(ActionType::Uri)
                    .uri(Str(url.as_bytes()));
            }

            annotations.finish();
            page_writer.finish();

            self.writer
                .stream(content_id, &deflate(&page.content.finish()))
                .filter(Filter::FlateDecode);
        }

        let mut pages = self.writer.pages(page_tree_ref);
        pages.count(page_refs.len() as i32).kids(page_refs);

        let mut resources = pages.resources();
        resources.color_spaces().insert(SRGB).start::<ColorSpace>().srgb();

        let mut fonts = resources.fonts();
        for (font_ref, f) in self.face_map.pdf_indices(&self.face_refs) {
            let name = format_eco!("F{}", f);
            fonts.pair(Name(name.as_bytes()), font_ref);
        }

        fonts.finish();

        let mut images = resources.x_objects();
        for (image_ref, im) in self.image_map.pdf_indices(&self.image_refs) {
            let name = format_eco!("Im{}", im);
            images.pair(Name(name.as_bytes()), image_ref);
        }

        images.finish();
        resources.finish();
        pages.finish();

        // Write the document information, catalog and wrap it up!
        self.writer.document_info(self.alloc.bump()).creator(TextStr("Typst"));
        self.writer.catalog(self.alloc.bump()).pages(page_tree_ref);
        self.writer.finish()
    }
}

/// An exporter for the contents of a single PDF page.
struct PageExporter<'a> {
    fonts: &'a FontStore,
    font_map: &'a mut Remapper<FaceId>,
    image_map: &'a mut Remapper<ImageId>,
    glyphs: &'a mut HashMap<FaceId, HashSet<u16>>,
    bottom: f32,
    content: Content,
    links: Vec<(String, Rect)>,
    state: State,
    saves: Vec<State>,
}

/// Data for an exported page.
struct Page {
    size: Size,
    content: Content,
    links: Vec<(String, Rect)>,
}

/// A simulated graphics state used to deduplicate graphics state changes and
/// keep track of the current transformation matrix for link annotations.
#[derive(Debug, Default, Clone)]
struct State {
    transform: Transform,
    fill: Option<Paint>,
    stroke: Option<Stroke>,
    font: Option<(FaceId, Length)>,
}

impl<'a> PageExporter<'a> {
    fn new(exporter: &'a mut PdfExporter) -> Self {
        Self {
            fonts: exporter.fonts,
            font_map: &mut exporter.face_map,
            image_map: &mut exporter.image_map,
            glyphs: &mut exporter.glyph_sets,
            bottom: 0.0,
            content: Content::new(),
            links: vec![],
            state: State::default(),
            saves: vec![],
        }
    }

    fn export(mut self, frame: &Frame) -> Page {
        // Make the coordinate system start at the top-left.
        self.bottom = frame.size.y.to_f32();
        self.content.transform([1.0, 0.0, 0.0, -1.0, 0.0, self.bottom]);
        self.content.set_fill_color_space(ColorSpaceOperand::Named(SRGB));
        self.content.set_stroke_color_space(ColorSpaceOperand::Named(SRGB));
        self.write_frame(frame);
        Page {
            size: frame.size,
            content: self.content,
            links: self.links,
        }
    }

    fn write_frame(&mut self, frame: &Frame) {
        for &(pos, ref element) in &frame.elements {
            let x = pos.x.to_f32();
            let y = pos.y.to_f32();
            match *element {
                Element::Group(ref group) => self.write_group(pos, group),
                Element::Text(ref text) => self.write_text(x, y, text),
                Element::Shape(ref shape) => self.write_shape(x, y, shape),
                Element::Image(id, size) => self.write_image(x, y, id, size),
                Element::Link(ref url, size) => self.write_link(pos, url, size),
            }
        }
    }

    fn write_group(&mut self, pos: Point, group: &Group) {
        let translation = Transform::translate(pos.x, pos.y);

        self.save_state();
        self.transform(translation.pre_concat(group.transform));

        if group.clips {
            let w = group.frame.size.x.to_f32();
            let h = group.frame.size.y.to_f32();
            self.content.move_to(0.0, 0.0);
            self.content.line_to(w, 0.0);
            self.content.line_to(w, h);
            self.content.line_to(0.0, h);
            self.content.clip_nonzero();
            self.content.end_path();
        }

        self.write_frame(&group.frame);
        self.restore_state();
    }

    fn write_text(&mut self, x: f32, y: f32, text: &Text) {
        self.glyphs
            .entry(text.face_id)
            .or_default()
            .extend(text.glyphs.iter().map(|g| g.id));

        self.content.begin_text();
        self.set_font(text.face_id, text.size);
        self.set_fill(text.fill);

        let face = self.fonts.get(text.face_id);

        // Position the text.
        self.content.set_text_matrix([1.0, 0.0, 0.0, -1.0, x, y]);

        let mut positioned = self.content.show_positioned();
        let mut items = positioned.items();
        let mut adjustment = Em::zero();
        let mut encoded = vec![];

        // Write the glyphs with kerning adjustments.
        for glyph in &text.glyphs {
            adjustment += glyph.x_offset;

            if !adjustment.is_zero() {
                if !encoded.is_empty() {
                    items.show(Str(&encoded));
                    encoded.clear();
                }

                items.adjust(-adjustment.to_font_units());
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
            items.show(Str(&encoded));
        }

        items.finish();
        positioned.finish();
        self.content.end_text();
    }

    fn write_shape(&mut self, x: f32, y: f32, shape: &Shape) {
        if shape.fill.is_none() && shape.stroke.is_none() {
            return;
        }

        match shape.geometry {
            Geometry::Rect(size) => {
                let w = size.x.to_f32();
                let h = size.y.to_f32();
                if w > 0.0 && h > 0.0 {
                    self.content.rect(x, y, w, h);
                }
            }
            Geometry::Ellipse(size) => {
                let approx = geom::Path::ellipse(size);
                self.write_path(x, y, &approx);
            }
            Geometry::Line(target) => {
                let dx = target.x.to_f32();
                let dy = target.y.to_f32();
                self.content.move_to(x, y);
                self.content.line_to(x + dx, y + dy);
            }
            Geometry::Path(ref path) => {
                self.write_path(x, y, path);
            }
        }

        if let Some(fill) = shape.fill {
            self.set_fill(fill);
        }

        if let Some(stroke) = shape.stroke {
            self.set_stroke(stroke);
        }

        match (shape.fill, shape.stroke) {
            (None, None) => unreachable!(),
            (Some(_), None) => self.content.fill_nonzero(),
            (None, Some(_)) => self.content.stroke(),
            (Some(_), Some(_)) => self.content.fill_nonzero_and_stroke(),
        };
    }

    fn write_path(&mut self, x: f32, y: f32, path: &geom::Path) {
        for elem in &path.0 {
            match elem {
                geom::PathElement::MoveTo(p) => {
                    self.content.move_to(x + p.x.to_f32(), y + p.y.to_f32())
                }
                geom::PathElement::LineTo(p) => {
                    self.content.line_to(x + p.x.to_f32(), y + p.y.to_f32())
                }
                geom::PathElement::CubicTo(p1, p2, p3) => self.content.cubic_to(
                    x + p1.x.to_f32(),
                    y + p1.y.to_f32(),
                    x + p2.x.to_f32(),
                    y + p2.y.to_f32(),
                    x + p3.x.to_f32(),
                    y + p3.y.to_f32(),
                ),
                geom::PathElement::ClosePath => self.content.close_path(),
            };
        }
    }

    fn write_image(&mut self, x: f32, y: f32, id: ImageId, size: Size) {
        self.image_map.insert(id);
        let name = format_eco!("Im{}", self.image_map.map(id));
        let w = size.x.to_f32();
        let h = size.y.to_f32();
        self.content.save_state();
        self.content.transform([w, 0.0, 0.0, -h, x, y + h]);
        self.content.x_object(Name(name.as_bytes()));
        self.content.restore_state();
    }

    fn write_link(&mut self, pos: Point, url: &str, size: Size) {
        let mut min_x = Length::inf();
        let mut min_y = Length::inf();
        let mut max_x = -Length::inf();
        let mut max_y = -Length::inf();

        // Compute the bounding box of the transformed link.
        for point in [
            pos,
            pos + Point::with_x(size.x),
            pos + Point::with_y(size.y),
            pos + size.to_point(),
        ] {
            let t = point.transform(self.state.transform);
            min_x.set_min(t.x);
            min_y.set_min(t.y);
            max_x.set_max(t.x);
            max_y.set_max(t.y);
        }

        let x1 = min_x.to_f32();
        let x2 = max_x.to_f32();
        let y1 = self.bottom - max_y.to_f32();
        let y2 = self.bottom - min_y.to_f32();
        let rect = Rect::new(x1, y1, x2, y2);
        self.links.push((url.to_string(), rect));
    }

    fn save_state(&mut self) {
        self.saves.push(self.state.clone());
        self.content.save_state();
    }

    fn restore_state(&mut self) {
        self.content.restore_state();
        self.state = self.saves.pop().expect("missing state save");
    }

    fn transform(&mut self, transform: Transform) {
        let Transform { sx, ky, kx, sy, tx, ty } = transform;
        self.state.transform = self.state.transform.pre_concat(transform);
        self.content.transform([
            sx.get() as _,
            ky.get() as _,
            kx.get() as _,
            sy.get() as _,
            tx.to_f32(),
            ty.to_f32(),
        ]);
    }

    fn set_font(&mut self, face_id: FaceId, size: Length) {
        if self.state.font != Some((face_id, size)) {
            self.font_map.insert(face_id);
            let name = format_eco!("F{}", self.font_map.map(face_id));
            self.content.set_font(Name(name.as_bytes()), size.to_f32());
        }
    }

    fn set_fill(&mut self, fill: Paint) {
        if self.state.fill != Some(fill) {
            let f = |c| c as f32 / 255.0;
            let Paint::Solid(color) = fill;
            match color {
                Color::Rgba(c) => {
                    self.content.set_fill_color([f(c.r), f(c.g), f(c.b)]);
                }
                Color::Cmyk(c) => {
                    self.content.set_fill_cmyk(f(c.c), f(c.m), f(c.y), f(c.k));
                }
            }
        }
    }

    fn set_stroke(&mut self, stroke: Stroke) {
        if self.state.stroke != Some(stroke) {
            let f = |c| c as f32 / 255.0;
            let Paint::Solid(color) = stroke.paint;
            match color {
                Color::Rgba(c) => {
                    self.content.set_stroke_color([f(c.r), f(c.g), f(c.b)]);
                }
                Color::Cmyk(c) => {
                    self.content.set_stroke_cmyk(f(c.c), f(c.m), f(c.y), f(c.k));
                }
            }

            self.content.set_line_width(stroke.thickness.to_f32());
        }
    }
}

/// Encode an image with a suitable filter and return the data, filter and
/// whether the image has color.
///
/// Skips the alpha channel as that's encoded separately.
fn encode_image(img: &RasterImage) -> ImageResult<(Vec<u8>, Filter, bool)> {
    Ok(match (img.format, &img.buf) {
        // 8-bit gray JPEG.
        (ImageFormat::Jpeg, DynamicImage::ImageLuma8(_)) => {
            let mut data = vec![];
            img.buf.write_to(&mut data, img.format)?;
            (data, Filter::DctDecode, false)
        }

        // 8-bit Rgb JPEG (Cmyk JPEGs get converted to Rgb earlier).
        (ImageFormat::Jpeg, DynamicImage::ImageRgb8(_)) => {
            let mut data = vec![];
            img.buf.write_to(&mut data, img.format)?;
            (data, Filter::DctDecode, true)
        }

        // TODO: Encode flate streams with PNG-predictor?

        // 8-bit gray PNG.
        (ImageFormat::Png, DynamicImage::ImageLuma8(luma)) => {
            let data = deflate(luma.as_raw());
            (data, Filter::FlateDecode, false)
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

            let data = deflate(&pixels);
            (data, Filter::FlateDecode, true)
        }
    })
}

/// Encode an image's alpha channel if present.
fn encode_alpha(img: &RasterImage) -> (Vec<u8>, Filter) {
    let pixels: Vec<_> = img.buf.pixels().map(|(_, _, Rgba([_, _, _, a]))| a).collect();
    (deflate(&pixels), Filter::FlateDecode)
}

/// Compress data with the DEFLATE algorithm.
fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

/// Assigns new, consecutive PDF-internal indices to things.
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

    fn pdf_indices<'a>(
        &'a self,
        refs: &'a [Ref],
    ) -> impl Iterator<Item = (Ref, usize)> + 'a {
        refs.iter().copied().zip(0 .. self.to_pdf.len())
    }

    fn layout_indices(&self) -> impl Iterator<Item = Index> + '_ {
        self.to_layout.iter().copied()
    }
}

/// Additional methods for [`Length`].
trait LengthExt {
    /// Convert an em length to a number of points.
    fn to_f32(self) -> f32;
}

impl LengthExt for Length {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

/// Additional methods for [`Em`].
trait EmExt {
    /// Convert an em length to a number of PDF font units.
    fn to_font_units(self) -> f32;
}

impl EmExt for Em {
    fn to_font_units(self) -> f32 {
        1000.0 * self.get() as f32
    }
}

/// Additional methods for [`Ref`].
trait RefExt {
    /// Bump the reference up by one and return the previous one.
    fn bump(&mut self) -> Self;
}

impl RefExt for Ref {
    fn bump(&mut self) -> Self {
        let prev = *self;
        *self = Self::new(prev.get() + 1);
        prev
    }
}

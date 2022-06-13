//! Exporting into PDF documents.

use std::cmp::Eq;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::io::Cursor;

use image::{DynamicImage, GenericImageView, ImageFormat, ImageResult, Rgba};
use pdf_writer::types::{
    ActionType, AnnotationType, CidFontType, ColorSpaceOperand, Direction, FontFlags,
    SystemInfo, UnicodeCmap,
};
use pdf_writer::writers::ColorSpace;
use pdf_writer::{Content, Filter, Finish, Name, PdfWriter, Rect, Ref, Str, TextStr};
use ttf_parser::{name_id, GlyphId, Tag};

use crate::font::{FaceId, FontStore};
use crate::frame::{Destination, Element, Frame, Group, Role, Text};
use crate::geom::{
    self, Color, Dir, Em, Geometry, Length, Numeric, Paint, Point, Ratio, Shape, Size,
    Stroke, Transform,
};
use crate::image::{Image, ImageId, ImageStore, RasterImage};
use crate::library::prelude::EcoString;
use crate::library::text::Lang;
use crate::util::SliceExt;
use crate::Context;

/// Export a collection of frames into a PDF file.
///
/// This creates one page per frame. In addition to the frames, you need to pass
/// in the context used during compilation so that fonts and images can be
/// included in the PDF.
///
/// Returns the raw bytes making up the PDF file.
pub fn pdf(ctx: &Context, frames: &[Frame]) -> Vec<u8> {
    PdfExporter::new(ctx).export(frames)
}

/// Identifies the color space definitions.
const SRGB: Name<'static> = Name(b"srgb");
const D65_GRAY: Name<'static> = Name(b"d65gray");

/// An exporter for a whole PDF document.
struct PdfExporter<'a> {
    writer: PdfWriter,
    fonts: &'a FontStore,
    images: &'a ImageStore,
    pages: Vec<Page>,
    page_heights: Vec<f32>,
    alloc: Ref,
    page_tree_ref: Ref,
    face_refs: Vec<Ref>,
    image_refs: Vec<Ref>,
    page_refs: Vec<Ref>,
    face_map: Remapper<FaceId>,
    image_map: Remapper<ImageId>,
    glyph_sets: HashMap<FaceId, HashSet<u16>>,
    languages: HashMap<Lang, usize>,
    heading_tree: Vec<HeadingNode>,
}

impl<'a> PdfExporter<'a> {
    fn new(ctx: &'a Context) -> Self {
        let mut alloc = Ref::new(1);
        let page_tree_ref = alloc.bump();
        Self {
            writer: PdfWriter::new(),
            fonts: &ctx.fonts,
            images: &ctx.images,
            pages: vec![],
            page_heights: vec![],
            alloc,
            page_tree_ref,
            page_refs: vec![],
            face_refs: vec![],
            image_refs: vec![],
            face_map: Remapper::new(),
            image_map: Remapper::new(),
            glyph_sets: HashMap::new(),
            languages: HashMap::new(),
            heading_tree: vec![],
        }
    }

    fn export(mut self, frames: &[Frame]) -> Vec<u8> {
        self.build_pages(frames);
        self.write_fonts();
        self.write_images();

        for page in std::mem::take(&mut self.pages).into_iter() {
            self.write_page(page);
        }

        self.write_page_tree();
        self.write_catalog();

        self.writer.finish()
    }

    fn build_pages(&mut self, frames: &[Frame]) {
        for frame in frames {
            let page_id = self.alloc.bump();
            self.page_refs.push(page_id);
            let page = PageExporter::new(self, page_id).export(frame);
            self.page_heights.push(page.size.y.to_f32());
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

            let postscript_name = face
                .find_name(name_id::POST_SCRIPT_NAME)
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
            let mut cid = self.writer.cid_font(cid_ref);
            cid.subtype(subtype);
            cid.base_font(base_font);
            cid.system_info(system_info);
            cid.font_descriptor(descriptor_ref);
            cid.default_width(0.0);

            if subtype == CidFontType::Type2 {
                cid.cid_to_gid_map_predefined(Name(b"Identity"));
            }

            // Extract the widths of all glyphs.
            let num_glyphs = ttf.number_of_glyphs();
            let mut widths = vec![0.0; num_glyphs as usize];
            for &g in glyphs {
                let x = ttf.glyph_hor_advance(GlyphId(g)).unwrap_or(0);
                widths[g as usize] = face.to_em(x).to_font_units();
            }

            // Write all non-zero glyph widths.
            let mut first = 0;
            let mut width_writer = cid.widths();
            for (w, group) in widths.group_by_key(|&w| w) {
                let end = first + group.len();
                if w != 0.0 {
                    let last = end - 1;
                    width_writer.same(first as u16, last as u16, w);
                }
                first = end;
            }

            width_writer.finish();
            cid.finish();

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
            let mut font_descriptor = self.writer.font_descriptor(descriptor_ref);
            font_descriptor
                .name(base_font)
                .flags(flags)
                .bbox(bbox)
                .italic_angle(italic_angle)
                .ascent(ascender)
                .descent(descender)
                .cap_height(cap_height)
                .stem_v(stem_v);

            match subtype {
                CidFontType::Type0 => font_descriptor.font_file3(data_ref),
                CidFontType::Type2 => font_descriptor.font_file2(data_ref),
            };

            font_descriptor.finish();

            // Compute a reverse mapping from glyphs to unicode.
            let cmap = {
                let mut mapping = BTreeMap::new();
                for subtable in
                    ttf.tables().cmap.into_iter().flat_map(|table| table.subtables)
                {
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
            let data = face.buffer();
            let subsetted = {
                let glyphs: Vec<_> = glyphs.iter().copied().collect();
                let profile = subsetter::Profile::pdf(&glyphs);
                subsetter::subset(data, face.index(), profile)
            };

            // Compress and write the face's byte.
            let data = subsetted.as_deref().unwrap_or(data);
            let data = deflate(data);
            let mut stream = self.writer.stream(data_ref, &data);
            stream.filter(Filter::FlateDecode);

            if subtype == CidFontType::Type0 {
                stream.pair(Name(b"Subtype"), Name(b"OpenType"));
            }

            stream.finish();
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

    fn write_page(&mut self, page: Page) {
        let content_id = self.alloc.bump();

        let mut page_writer = self.writer.page(page.id);
        page_writer.parent(self.page_tree_ref);

        let w = page.size.x.to_f32();
        let h = page.size.y.to_f32();
        page_writer.media_box(Rect::new(0.0, 0.0, w, h));
        page_writer.contents(content_id);

        let mut annotations = page_writer.annotations();
        for (dest, rect) in page.links {
            let mut link = annotations.push();
            link.subtype(AnnotationType::Link).rect(rect);
            match dest {
                Destination::Url(uri) => {
                    link.action()
                        .action_type(ActionType::Uri)
                        .uri(Str(uri.as_str().as_bytes()));
                }
                Destination::Internal(loc) => {
                    let index = loc.page.get() - 1;
                    if let Some(&height) = self.page_heights.get(index) {
                        link.action()
                            .action_type(ActionType::GoTo)
                            .destination_direct()
                            .page(self.page_refs[index])
                            .xyz(loc.pos.x.to_f32(), height - loc.pos.y.to_f32(), None);
                    }
                }
            }
        }

        annotations.finish();
        page_writer.finish();

        let data = page.content.finish();
        let data = deflate(&data);
        self.writer.stream(content_id, &data).filter(Filter::FlateDecode);
    }

    fn write_page_tree(&mut self) {
        let mut pages = self.writer.pages(self.page_tree_ref);
        pages
            .count(self.page_refs.len() as i32)
            .kids(self.page_refs.iter().copied());

        let mut resources = pages.resources();
        let mut spaces = resources.color_spaces();
        spaces.insert(SRGB).start::<ColorSpace>().srgb();
        spaces.insert(D65_GRAY).start::<ColorSpace>().d65_gray();
        spaces.finish();

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
    }

    fn write_catalog(&mut self) {
        // Build the outline tree.
        let outline_root_id = (!self.heading_tree.is_empty()).then(|| self.alloc.bump());
        let outline_start_ref = self.alloc;
        let len = self.heading_tree.len();
        let mut prev_ref = None;

        for (i, node) in std::mem::take(&mut self.heading_tree).iter().enumerate() {
            prev_ref = Some(self.write_outline_item(
                node,
                outline_root_id.unwrap(),
                prev_ref,
                i + 1 == len,
            ));
        }

        if let Some(outline_root_id) = outline_root_id {
            let mut outline_root = self.writer.outline(outline_root_id);
            outline_root.first(outline_start_ref);
            outline_root.last(Ref::new(self.alloc.get() - 1));
            outline_root.count(self.heading_tree.len() as i32);
        }

        let lang = self
            .languages
            .iter()
            .max_by_key(|(&lang, &count)| (count, lang))
            .map(|(&k, _)| k);

        let dir = if lang.map(Lang::dir) == Some(Dir::RTL) {
            Direction::R2L
        } else {
            Direction::L2R
        };

        // Write the document information.
        self.writer.document_info(self.alloc.bump()).creator(TextStr("Typst"));

        // Write the document catalog.
        let mut catalog = self.writer.catalog(self.alloc.bump());
        catalog.pages(self.page_tree_ref);
        catalog.viewer_preferences().direction(dir);

        if let Some(outline_root_id) = outline_root_id {
            catalog.outlines(outline_root_id);
        }

        if let Some(lang) = lang {
            catalog.lang(TextStr(lang.as_str()));
        }

        catalog.finish();
    }

    fn write_outline_item(
        &mut self,
        node: &HeadingNode,
        parent_ref: Ref,
        prev_ref: Option<Ref>,
        is_last: bool,
    ) -> Ref {
        let id = self.alloc.bump();
        let next_ref = Ref::new(id.get() + node.len() as i32);

        let mut outline = self.writer.outline_item(id);
        outline.parent(parent_ref);

        if !is_last {
            outline.next(next_ref);
        }

        if let Some(prev_rev) = prev_ref {
            outline.prev(prev_rev);
        }

        if !node.children.is_empty() {
            let current_child = Ref::new(id.get() + 1);
            outline.first(current_child);
            outline.last(Ref::new(next_ref.get() - 1));
            outline.count(-1 * node.children.len() as i32);
        }

        outline.title(TextStr(&node.heading.content));
        outline.dest_direct().page(node.heading.page).xyz(
            node.heading.position.x.to_f32(),
            (node.heading.position.y + Length::pt(3.0)).to_f32(),
            None,
        );

        outline.finish();

        let mut prev_ref = None;
        for (i, child) in node.children.iter().enumerate() {
            prev_ref = Some(self.write_outline_item(
                child,
                id,
                prev_ref,
                i + 1 == node.children.len(),
            ));
        }

        id
    }
}

/// An exporter for the contents of a single PDF page.
struct PageExporter<'a, 'b> {
    exporter: &'a mut PdfExporter<'b>,
    page_ref: Ref,
    content: Content,
    state: State,
    saves: Vec<State>,
    bottom: f32,
    links: Vec<(Destination, Rect)>,
}

/// Data for an exported page.
struct Page {
    id: Ref,
    size: Size,
    content: Content,
    links: Vec<(Destination, Rect)>,
}

/// A simulated graphics state used to deduplicate graphics state changes and
/// keep track of the current transformation matrix for link annotations.
#[derive(Debug, Default, Clone)]
struct State {
    transform: Transform,
    font: Option<(FaceId, Length)>,
    fill: Option<Paint>,
    fill_space: Option<Name<'static>>,
    stroke: Option<Stroke>,
    stroke_space: Option<Name<'static>>,
}

impl<'a, 'b> PageExporter<'a, 'b> {
    fn new(exporter: &'a mut PdfExporter<'b>, page_ref: Ref) -> Self {
        Self {
            exporter,
            page_ref,
            content: Content::new(),
            state: State::default(),
            saves: vec![],
            bottom: 0.0,
            links: vec![],
        }
    }

    fn export(mut self, frame: &Frame) -> Page {
        let size = frame.size();

        // Make the coordinate system start at the top-left.
        self.bottom = size.y.to_f32();
        self.transform(Transform {
            sx: Ratio::one(),
            ky: Ratio::zero(),
            kx: Ratio::zero(),
            sy: Ratio::new(-1.0),
            tx: Length::zero(),
            ty: size.y,
        });

        // Encode the page into the content stream.
        self.write_frame(frame);

        Page {
            size,
            content: self.content,
            id: self.page_ref,
            links: self.links,
        }
    }

    fn write_frame(&mut self, frame: &Frame) {
        if let Some(Role::Heading { level, outlined: true }) = frame.role() {
            let heading = Heading {
                position: Point::new(self.state.transform.tx, self.state.transform.ty),
                content: frame.text(),
                page: self.page_ref,
                level: level.get(),
            };

            if let Some(last) = self.exporter.heading_tree.last_mut() {
                if !last.insert(heading.clone(), 1) {
                    self.exporter.heading_tree.push(HeadingNode::leaf(heading))
                }
            } else {
                self.exporter.heading_tree.push(HeadingNode::leaf(heading))
            }
        }

        for &(pos, ref element) in frame.elements() {
            let x = pos.x.to_f32();
            let y = pos.y.to_f32();
            match *element {
                Element::Group(ref group) => self.write_group(pos, group),
                Element::Text(ref text) => self.write_text(x, y, text),
                Element::Shape(ref shape) => self.write_shape(x, y, shape),
                Element::Image(id, size) => self.write_image(x, y, id, size),
                Element::Link(ref dest, size) => self.write_link(pos, dest, size),
                Element::Pin(_) => {}
            }
        }
    }

    fn write_group(&mut self, pos: Point, group: &Group) {
        let translation = Transform::translate(pos.x, pos.y);

        self.save_state();
        self.transform(translation.pre_concat(group.transform));

        if group.clips {
            let size = group.frame.size();
            let w = size.x.to_f32();
            let h = size.y.to_f32();
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
        *self.exporter.languages.entry(text.lang).or_insert(0) += text.glyphs.len();
        self.exporter
            .glyph_sets
            .entry(text.face_id)
            .or_default()
            .extend(text.glyphs.iter().map(|g| g.id));

        let face = self.exporter.fonts.get(text.face_id);

        self.set_fill(text.fill);
        self.set_font(text.face_id, text.size);
        self.content.begin_text();

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

        if let Some(fill) = shape.fill {
            self.set_fill(fill);
        }

        if let Some(stroke) = shape.stroke {
            self.set_stroke(stroke);
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
        self.exporter.image_map.insert(id);
        let name = format_eco!("Im{}", self.exporter.image_map.map(id));
        let w = size.x.to_f32();
        let h = size.y.to_f32();
        self.content.save_state();
        self.content.transform([w, 0.0, 0.0, -h, x, y + h]);
        self.content.x_object(Name(name.as_bytes()));
        self.content.restore_state();
    }

    fn write_link(&mut self, pos: Point, dest: &Destination, size: Size) {
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
        let y1 = max_y.to_f32();
        let y2 = min_y.to_f32();
        let rect = Rect::new(x1, y1, x2, y2);

        self.links.push((dest.clone(), rect));
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
            self.exporter.face_map.insert(face_id);
            let name = format_eco!("F{}", self.exporter.face_map.map(face_id));
            self.content.set_font(Name(name.as_bytes()), size.to_f32());
            self.state.font = Some((face_id, size));
        }
    }

    fn set_fill(&mut self, fill: Paint) {
        if self.state.fill != Some(fill) {
            let f = |c| c as f32 / 255.0;
            let Paint::Solid(color) = fill;
            match color {
                Color::Luma(c) => {
                    self.set_fill_color_space(D65_GRAY);
                    self.content.set_fill_gray(f(c.0));
                }
                Color::Rgba(c) => {
                    self.set_fill_color_space(SRGB);
                    self.content.set_fill_color([f(c.r), f(c.g), f(c.b)]);
                }
                Color::Cmyk(c) => {
                    self.content.set_fill_cmyk(f(c.c), f(c.m), f(c.y), f(c.k));
                }
            }
            self.state.fill = Some(fill);
        }
    }

    fn set_fill_color_space(&mut self, space: Name<'static>) {
        if self.state.fill_space != Some(space) {
            self.content.set_fill_color_space(ColorSpaceOperand::Named(space));
            self.state.fill_space = Some(space);
        }
    }

    fn set_stroke(&mut self, stroke: Stroke) {
        if self.state.stroke != Some(stroke) {
            let f = |c| c as f32 / 255.0;
            let Paint::Solid(color) = stroke.paint;
            match color {
                Color::Luma(c) => {
                    self.set_stroke_color_space(D65_GRAY);
                    self.content.set_stroke_gray(f(c.0));
                }
                Color::Rgba(c) => {
                    self.set_stroke_color_space(SRGB);
                    self.content.set_stroke_color([f(c.r), f(c.g), f(c.b)]);
                }
                Color::Cmyk(c) => {
                    self.content.set_stroke_cmyk(f(c.c), f(c.m), f(c.y), f(c.k));
                }
            }

            self.content.set_line_width(stroke.thickness.to_f32());
            self.state.stroke = Some(stroke);
        }
    }

    fn set_stroke_color_space(&mut self, space: Name<'static>) {
        if self.state.stroke_space != Some(space) {
            self.content.set_stroke_color_space(ColorSpaceOperand::Named(space));
            self.state.stroke_space = Some(space);
        }
    }
}

/// A heading that can later be linked in the outline panel.
#[derive(Debug, Clone)]
struct Heading {
    content: EcoString,
    level: usize,
    position: Point,
    page: Ref,
}

/// A node in the outline tree.
#[derive(Debug, Clone)]
struct HeadingNode {
    heading: Heading,
    children: Vec<HeadingNode>,
}

impl HeadingNode {
    fn leaf(heading: Heading) -> Self {
        HeadingNode { heading, children: Vec::new() }
    }

    fn len(&self) -> usize {
        1 + self.children.iter().map(Self::len).sum::<usize>()
    }

    fn insert(&mut self, other: Heading, level: usize) -> bool {
        if level >= other.level {
            return false;
        }

        if let Some(child) = self.children.last_mut() {
            if child.insert(other.clone(), level + 1) {
                return true;
            }
        }

        self.children.push(Self::leaf(other));
        true
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
            let mut data = Cursor::new(vec![]);
            img.buf.write_to(&mut data, img.format)?;
            (data.into_inner(), Filter::DctDecode, false)
        }

        // 8-bit RGB JPEG (CMYK JPEGs get converted to RGB earlier).
        (ImageFormat::Jpeg, DynamicImage::ImageRgb8(_)) => {
            let mut data = Cursor::new(vec![]);
            img.buf.write_to(&mut data, img.format)?;
            (data.into_inner(), Filter::DctDecode, true)
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

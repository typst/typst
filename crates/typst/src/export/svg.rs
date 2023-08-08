use std::{
    collections::HashMap,
    fmt::{Display, Write},
    hash::Hash,
    io::Read,
};

use base64::Engine;
use ecow::{eco_format, EcoString};
use ttf_parser::{GlyphId, OutlineBuilder};
use xmlwriter::XmlWriter;

use crate::{
    doc::{Document, Frame, FrameItem, Glyph, GroupItem, TextItem},
    font::Font,
    geom::{Abs, Axes, Geometry, LineCap, LineJoin, PathItem, Ratio, Shape, Transform},
    image::{ImageFormat, RasterFormat, VectorFormat},
    util::hash128,
};
use crate::{geom::Paint::Solid, image::Image};

/// [`RenderHash`] is a hash value for a rendered glyph or clip path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RenderHash(u128);

/// Convert a [`u128`] into a [`RenderHash`].
impl From<u128> for RenderHash {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

/// Export a document into a SVG file.
#[tracing::instrument(skip_all)]
pub fn svg(doc: &Document) -> String {
    let mut renderer = SVGRenderer::new();
    let max_page_width = doc
        .pages
        .iter()
        .map(|page| page.size().x)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(Abs::zero());
    let total_page_height = doc.pages.iter().map(|page| page.size().y).sum::<Abs>();
    let doc_size = Axes { x: max_page_width, y: total_page_height };
    renderer.header(doc_size);
    let mut y_offset = Abs::zero();
    for page in &doc.pages {
        renderer.render_frame(page, Transform::translate(Abs::zero(), y_offset));
        y_offset += page.size().y;
    }
    renderer.finalize()
}

/// Export a frame into a SVG file.
#[tracing::instrument(skip_all)]
pub fn svg_frame(frame: &Frame) -> String {
    let mut renderer = SVGRenderer::new();
    renderer.header(frame.size());
    renderer.render_frame(frame, Transform::identity());
    renderer.finalize()
}

/// [`RenderedGlyph`] represet glyph to be rendered.
enum RenderedGlyph {
    /// A path is a sequence of drawing commands.
    /// It is in the format of `M x y L x y C x1 y1 x2 y2 x y Z`.
    Path(EcoString),
    /// An image is a URL to an image file, plus the size and transform. The url is in the
    /// format of `data:image/{format};base64,`.
    Image { url: EcoString, width: f64, height: f64, ts: Transform },
}

/// [`DedupVec`] is a vector that deduplicates its elements. It is used to deduplicate glyphs and
/// clip paths.
/// The `H` is the hash type, and `T` is the value type. The `PREFIX` is the prefix of the index.
/// This is used to distinguish between glyphs and clip paths.
#[derive(Debug, Clone)]
struct DedupVec<H, T, const PREFIX: char> {
    vec: Vec<T>,
    present: HashMap<H, usize>,
}

impl<H, T, const PREFIX: char> DedupVec<H, T, PREFIX>
where
    H: Eq + Hash + Copy,
{
    fn new() -> Self {
        Self { vec: Vec::new(), present: HashMap::new() }
    }

    /// Insert a value into the vector. If the value is already present, return the index of the
    /// existing value. And the value_fn will not be called. Otherwise, insert the value and
    /// return the index of the inserted value. The index is the position of the value in the
    /// vector.
    #[must_use = "This method returns the index of the inserted value"]
    fn insert_with(&mut self, hash: H, value_fn: impl FnOnce() -> T) -> usize {
        if let Some(index) = self.present.get(&hash) {
            *index
        } else {
            let index = self.vec.len();
            self.vec.push(value_fn());
            self.present.insert(hash, index);
            index
        }
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter()
    }

    fn prefix(&self) -> char {
        PREFIX
    }
}

impl<H, T, const PREFIX: char> IntoIterator for DedupVec<H, T, PREFIX> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}

/// [`SVGRenderer`] is a renderer that renders a document or frame into a SVG file.
struct SVGRenderer {
    xml: XmlWriter,
    glyphs: DedupVec<RenderHash, RenderedGlyph, 'g'>,
    /// Clip paths are used to clip a group. A clip path is a path that defines the clipping
    /// region. The clip path is referenced by the `clip-path` attribute of the group.
    /// The clip path is in the format of `M x y L x y C x1 y1 x2 y2 x y Z`.
    clip_paths: DedupVec<RenderHash, EcoString, 'c'>,
}

impl SVGRenderer {
    /// Create a new SVG renderer with empty glyph and clip path.
    fn new() -> Self {
        SVGRenderer {
            xml: XmlWriter::new(xmlwriter::Options::default()),
            glyphs: DedupVec::new(),
            clip_paths: DedupVec::new(),
        }
    }

    /// Write the SVG header, including the `viewBox` and `width` and `height` attributes.
    fn header(&mut self, size: Axes<Abs>) {
        self.xml.start_element("svg");
        self.xml.write_attribute("class", "typst-doc");
        self.xml.write_attribute_fmt(
            "viewBox",
            format_args!("0 0 {} {}", size.x.to_pt(), size.y.to_pt()),
        );
        self.xml.write_attribute("width", &size.x.to_pt());
        self.xml.write_attribute("height", &size.y.to_pt());
        self.xml.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        self.xml
            .write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        self.xml.write_attribute("xmlns:h5", "http://www.w3.org/1999/xhtml");
    }

    /// Build the glyph definitions.
    fn build_glyph(&mut self) {
        self.xml.start_element("defs");
        self.xml.write_attribute("id", "glyph");
        for (id, glyph) in self.glyphs.iter().enumerate() {
            self.xml.start_element("symbol");
            self.xml.write_attribute_fmt(
                "id",
                format_args!("{}{}", self.glyphs.prefix(), id),
            );
            self.xml.write_attribute("overflow", "visible");
            match glyph {
                RenderedGlyph::Path(path) => {
                    self.xml.start_element("path");
                    self.xml.write_attribute("d", &path);
                    self.xml.end_element();
                }
                RenderedGlyph::Image { url, width, height, ts } => {
                    self.xml.start_element("image");
                    self.xml.write_attribute("xlink:href", &url);
                    self.xml.write_attribute("width", &width);
                    self.xml.write_attribute("height", &height);
                    if !ts.is_identity() {
                        self.xml.write_attribute("transform", &ts);
                    }
                    self.xml.write_attribute("preserveAspectRatio", "none");
                    self.xml.end_element();
                }
            }
            self.xml.end_element();
        }
        self.xml.end_element();
    }

    /// Build the clip path definitions.
    fn build_clip_path(&mut self) {
        self.xml.start_element("defs");
        self.xml.write_attribute("id", "clip-path");
        for (id, path) in self.clip_paths.iter().enumerate() {
            self.xml.start_element("clipPath");
            self.xml.write_attribute_fmt(
                "id",
                format_args!("{}{}", self.clip_paths.prefix(), id),
            );
            self.xml.start_element("path");
            self.xml.write_attribute("d", &path);
            self.xml.end_element();
            self.xml.end_element();
        }
        self.xml.end_element();
    }

    /// Finalize the SVG file. This must be called after all rendering is done.
    fn finalize(mut self) -> String {
        self.build_clip_path();
        self.build_glyph();
        self.xml.end_document()
    }

    /// Render a frame with the given transform.
    fn render_frame(&mut self, frame: &Frame, ts: Transform) {
        self.xml.start_element("g");
        if !ts.is_identity() {
            self.xml.write_attribute("transform", &ts);
        };
        for (pos, item) in frame.items() {
            let x = pos.x.to_pt();
            let y = pos.y.to_pt();
            self.xml.start_element("g");
            self.xml
                .write_attribute_fmt("transform", format_args!("translate({} {})", x, y));
            match item {
                FrameItem::Group(group) => self.render_group(group),
                FrameItem::Text(text) => self.render_text(text),
                FrameItem::Shape(shape, _) => self.render_shape(shape),
                FrameItem::Image(image, size, _) => self.render_image(image, size),
                FrameItem::Meta(_, _) => {}
            };
            self.xml.end_element();
        }
        self.xml.end_element();
    }

    /// Render a group. If the group has `clips` set to true, a clip path will be created.
    fn render_group(&mut self, group: &GroupItem) {
        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-group");
        if group.clips {
            let clip_path_hash = hash128(&group).into();
            let x = group.frame.size().x.to_pt();
            let y = group.frame.size().y.to_pt();
            let id = self.clip_paths.insert_with(clip_path_hash, || {
                let mut builder = SVGPath2DBuilder(EcoString::new());
                builder.rect(x as f32, y as f32);
                builder.0
            });
            self.xml.write_attribute_fmt(
                "clip-path",
                format_args!("url(#{}{})", self.clip_paths.prefix(), id),
            );
        }
        self.render_frame(&group.frame, group.transform);
        self.xml.end_element();
    }

    /// Render a text item. The text is rendered as a group of glyphs.
    /// We will try to render the text as SVG first, then bitmap, then outline.
    /// If none of them works, we will skip the text.
    fn render_text(&mut self, text: &TextItem) {
        let scale: f64 = text.size.to_pt() / text.font.units_per_em();
        let inv_scale: f64 = text.font.units_per_em() / text.size.to_pt();
        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-text");
        self.xml.write_attribute_fmt(
            "transform",
            format_args!("scale({} {})", scale, -scale),
        );
        let mut x_offset: f64 = 0.0;
        for glyph in &text.glyphs {
            let offset = x_offset + glyph.x_offset.at(text.size).to_pt();
            self.render_svg_glyph(text, glyph, offset, inv_scale)
                .or_else(|| self.render_bitmap_glyph(text, glyph, offset, inv_scale))
                .or_else(|| self.render_outline_glyph(text, glyph, offset, inv_scale));
            x_offset += glyph.x_advance.at(text.size).to_pt();
        }
        self.xml.end_element();
    }

    fn render_svg_glyph(
        &mut self,
        text: &TextItem,
        glyph: &Glyph,
        x_offset: f64,
        inv_scale: f64,
    ) -> Option<()> {
        #[comemo::memoize]
        fn build_svg_glyph(font: &Font, glyph_id: u16) -> Option<EcoString> {
            let mut data = font.ttf().glyph_svg_image(GlyphId(glyph_id))?;
            // Decompress SVGZ.
            let mut decoded = vec![];
            // The first three bytes of the gzip-encoded document header must be 0x1F, 0x8B,
            // 0x08.
            if data.starts_with(&[0x1f, 0x8b]) {
                let mut decoder = flate2::read::GzDecoder::new(data);
                decoder.read_to_end(&mut decoded).ok()?;
                data = &decoded;
            }

            let upem = Abs::raw(font.units_per_em());
            let (width, height) = (upem.to_pt(), upem.to_pt());
            let origin_ascender = font.metrics().ascender.at(upem).to_pt();

            // Parse XML.
            let mut svg_str = std::str::from_utf8(data).ok()?.to_owned();
            let document = xmlparser::Tokenizer::from(svg_str.as_str());
            let mut start_span = None;
            let mut last_viewbox = None;
            // Parse xml and find the viewBox of the svg element.
            // <svg viewBox="0 0 1000 1000">...</svg>
            // ~~~~~^~~~~~~
            for n in document {
                let tok = n.unwrap();
                match tok {
                    xmlparser::Token::ElementStart { span, local, .. } => {
                        if local.as_str() == "svg" {
                            start_span = Some(span);
                            break;
                        }
                    }
                    xmlparser::Token::Attribute { span, local, value, .. } => {
                        if local.as_str() == "viewBox" {
                            last_viewbox = Some((span, value));
                        }
                    }
                    xmlparser::Token::ElementEnd { .. } => break,
                    _ => {}
                }
            }

            if last_viewbox.is_none() {
                // correct the viewbox if it is not present
                // `-origin_ascender` is to make sure the glyph is rendered at the correct position
                svg_str.insert_str(
                    start_span.unwrap().range().end,
                    format!(r#" viewBox="0 {} {} {}""#, -origin_ascender, width, height)
                        .as_str(),
                );
            }
            let mut url: EcoString = "data:image/svg+xml;base64,".into();
            let b64_encoded =
                base64::engine::general_purpose::STANDARD.encode(svg_str.as_bytes());
            url.push_str(&b64_encoded);
            Some(url)
        }

        let data_url = build_svg_glyph(&text.font, glyph.id)?;
        let upem = Abs::raw(text.font.units_per_em());
        let origin_ascender = text.font.metrics().ascender.at(upem).to_pt();
        let glyph_hash: RenderHash = hash128(&(&text.font, glyph.id)).into();
        let id = self.glyphs.insert_with(glyph_hash, || RenderedGlyph::Image {
            url: data_url,
            width: upem.to_pt(),
            height: upem.to_pt(),
            ts: Transform::translate(Abs::zero(), Abs::pt(-origin_ascender))
                .post_concat(Transform::scale(Ratio::new(1.0), Ratio::new(-1.0))),
        });

        self.xml.start_element("use");
        self.xml.write_attribute_fmt(
            "xlink:href",
            format_args!("#{}{}", self.glyphs.prefix(), id),
        );
        self.xml
            .write_attribute_fmt("x", format_args!("{}", x_offset * inv_scale));
        self.xml.end_element();
        Some(())
    }

    fn render_bitmap_glyph(
        &mut self,
        text: &TextItem,
        glyph: &Glyph,
        x_offset: f64,
        inv_scale: f64,
    ) -> Option<()> {
        #[comemo::memoize]
        fn build_bitmap_glyph(font: &Font, glyph_id: u16) -> Option<(Image, i16, i16)> {
            let bitmap =
                font.ttf().glyph_raster_image(GlyphId(glyph_id), std::u16::MAX)?;
            let image =
                Image::new(bitmap.data.into(), bitmap.format.into(), None).ok()?;
            Some((image, bitmap.x, bitmap.y))
        }
        let glyph_hash: RenderHash = hash128(&(&text.font, glyph.id)).into();
        let (image, bitmap_x_offset, bitmap_y_offset) =
            build_bitmap_glyph(&text.font, glyph.id)?;
        let (bitmap_x_offset, bitmap_y_offset) =
            (bitmap_x_offset as f64, bitmap_y_offset as f64);
        let id = self.glyphs.insert_with(glyph_hash, || {
            let width = image.width() as f64;
            let height = image.height() as f64;
            let url = encode_image_to_url(&image);
            let ts = Transform::translate(
                Abs::pt(bitmap_x_offset),
                Abs::pt(-height - bitmap_y_offset),
            );
            RenderedGlyph::Image { url, width, height, ts }
        });
        let target_height = text.size.to_pt();
        self.xml.start_element("use");
        self.xml.write_attribute_fmt(
            "xlink:href",
            format_args!("#{}{}", self.glyphs.prefix(), id),
        );
        // The image is stored with the height of `image.height()`, but we want to render it with a
        // height of `target_height`. So we need to scale it.
        let scale_factor = target_height / image.height() as f64;
        self.xml.write_attribute("x", &(x_offset / scale_factor));
        self.xml.write_attribute_fmt(
            "transform",
            format_args!(
                "scale({} -{})",
                inv_scale * scale_factor,
                inv_scale * scale_factor,
            ),
        );
        self.xml.end_element();
        Some(())
    }

    fn render_outline_glyph(
        &mut self,
        text: &TextItem,
        glyph: &Glyph,
        x_offset: f64,
        inv_scale: f64,
    ) -> Option<()> {
        #[comemo::memoize]
        fn build_outline_glyph(font: &Font, glyph_id: u16) -> Option<EcoString> {
            let mut builder = SVGPath2DBuilder(EcoString::new());
            font.ttf().outline_glyph(GlyphId(glyph_id), &mut builder)?;
            Some(builder.0)
        }
        let path = build_outline_glyph(&text.font, glyph.id)?;
        let glyph_hash = hash128(&(&text.font, glyph.id)).into();
        let id = self.glyphs.insert_with(glyph_hash, || RenderedGlyph::Path(path));
        let Solid(text_color) = text.fill;
        self.xml.start_element("use");
        self.xml.write_attribute_fmt(
            "xlink:href",
            format_args!("#{}{}", self.glyphs.prefix(), id),
        );
        self.xml
            .write_attribute_fmt("x", format_args!("{}", x_offset * inv_scale));
        self.xml.write_attribute("fill", &text_color.to_rgba().to_hex());
        self.xml.end_element();
        Some(())
    }

    fn render_shape(&mut self, shape: &Shape) {
        self.xml.start_element("path");
        self.xml.write_attribute("class", "typst-shape");
        if let Some(paint) = &shape.fill {
            let Solid(color) = paint;
            self.xml.write_attribute("fill", &color.to_rgba().to_hex());
        } else {
            self.xml.write_attribute("fill", "none");
        }
        if let Some(stroke) = &shape.stroke {
            let Solid(color) = stroke.paint;
            self.xml.write_attribute("stroke", &color.to_rgba().to_hex());
            self.xml.write_attribute("stroke-width", &stroke.thickness.to_pt());
            self.xml.write_attribute(
                "stroke-linecap",
                match stroke.line_cap {
                    LineCap::Butt => "butt",
                    LineCap::Round => "round",
                    LineCap::Square => "square",
                },
            );
            self.xml.write_attribute(
                "stoke-linejoin",
                match stroke.line_join {
                    LineJoin::Miter => "miter",
                    LineJoin::Round => "round",
                    LineJoin::Bevel => "bevel",
                },
            );
            self.xml.write_attribute("stoke-miterlimit", &stroke.miter_limit.0);
            if let Some(pattern) = &stroke.dash_pattern {
                self.xml.write_attribute("stoken-dashoffset", &pattern.phase.to_pt());
                self.xml.write_attribute(
                    "stoken-dasharray",
                    &pattern
                        .array
                        .iter()
                        .map(|dash| dash.to_pt().to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
        }
        #[comemo::memoize]
        fn build_shape(geometry: &Geometry) -> EcoString {
            let mut path_builder = SVGPath2DBuilder(EcoString::new());
            match geometry {
                Geometry::Line(t) => {
                    path_builder.move_to(0.0, 0.0);
                    path_builder.line_to(t.x.to_pt() as f32, t.y.to_pt() as f32);
                }
                Geometry::Rect(rect) => {
                    let x = rect.x.to_pt() as f32;
                    let y = rect.y.to_pt() as f32;
                    path_builder.rect(x, y);
                }
                Geometry::Path(p) => {
                    for item in &p.0 {
                        match item {
                            PathItem::MoveTo(m) => path_builder
                                .move_to(m.x.to_pt() as f32, m.y.to_pt() as f32),
                            PathItem::LineTo(l) => path_builder
                                .line_to(l.x.to_pt() as f32, l.y.to_pt() as f32),
                            PathItem::CubicTo(c1, c2, t) => path_builder.curve_to(
                                c1.x.to_pt() as f32,
                                c1.y.to_pt() as f32,
                                c2.x.to_pt() as f32,
                                c2.y.to_pt() as f32,
                                t.x.to_pt() as f32,
                                t.y.to_pt() as f32,
                            ),
                            PathItem::ClosePath => path_builder.close(),
                        }
                    }
                }
            };
            path_builder.0
        }
        let shape_path = build_shape(&shape.geometry);
        self.xml.write_attribute("d", &shape_path);
        self.xml.end_element();
    }

    fn render_image(&mut self, image: &Image, size: &Axes<Abs>) {
        let url = encode_image_to_url(image);
        self.xml.start_element("image");
        self.xml.write_attribute("xlink:href", &url);
        self.xml.write_attribute("width", &size.x.to_pt());
        self.xml.write_attribute("height", &size.y.to_pt());
        self.xml.write_attribute("preserveAspectRatio", "none");
        self.xml.end_element();
    }
}

/// Encode an image into a data URL. The format of the URL is `data:image/{format};base64,`.
#[comemo::memoize]
fn encode_image_to_url(image: &Image) -> EcoString {
    let format = match image.format() {
        ImageFormat::Raster(f) => match f {
            RasterFormat::Png => "png",
            RasterFormat::Jpg => "jpeg",
            RasterFormat::Gif => "gif",
        },
        ImageFormat::Vector(f) => match f {
            VectorFormat::Svg => "svg+xml",
        },
    };
    let mut url = eco_format!("data:image/{};base64,", format);
    let data = base64::engine::general_purpose::STANDARD.encode(image.data());
    url.push_str(&data);
    url
}

impl Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert a [`Transform`] into a SVG transform string.
        // See https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform
        write!(
            f,
            "matrix({} {} {} {} {} {})",
            self.sx.get(),
            self.ky.get(),
            self.kx.get(),
            self.sy.get(),
            self.tx.to_pt(),
            self.ty.to_pt()
        )
    }
}
/// A builder for SVG path.
struct SVGPath2DBuilder(pub EcoString);

impl SVGPath2DBuilder {
    /// Create a rectangle path. The rectangle is created with the top-left corner at (0, 0).
    /// The width and height are the size of the rectangle.
    fn rect(&mut self, width: f32, height: f32) {
        self.move_to(0.0, 0.0);
        self.line_to(0.0, height);
        self.line_to(width, height);
        self.line_to(width, 0.0);
        self.close();
    }
}

/// A builder for SVG path. This is used to build the path for a glyph.
impl ttf_parser::OutlineBuilder for SVGPath2DBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(&mut self.0, "M {} {} ", x, y).unwrap();
    }

    fn line_to(&mut self, x: f32, y: f32) {
        write!(&mut self.0, "L {} {} ", x, y).unwrap();
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        write!(&mut self.0, "Q {} {} {} {} ", x1, y1, x, y).unwrap();
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        write!(&mut self.0, "C {} {} {} {} {} {} ", x1, y1, x2, y2, x, y).unwrap();
    }

    fn close(&mut self) {
        write!(&mut self.0, "Z ").unwrap();
    }
}

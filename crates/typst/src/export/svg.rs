use std::collections::HashMap;
use std::fmt::{self, Display, Formatter, Write};
use std::io::Read;

use base64::Engine;
use ecow::{eco_format, EcoString};
use ttf_parser::{GlyphId, OutlineBuilder};
use xmlwriter::XmlWriter;

use crate::doc::{Frame, FrameItem, GroupItem, TextItem};
use crate::font::Font;
use crate::geom::{
    Abs, Axes, Geometry, LineCap, LineJoin, Paint, PathItem, Ratio, Shape, Size, Stroke,
    Transform,
};
use crate::image::{Image, ImageFormat, RasterFormat, VectorFormat};
use crate::util::hash128;

/// Export a frame into a SVG file.
#[tracing::instrument(skip_all)]
pub fn svg(frame: &Frame) -> String {
    let mut renderer = SVGRenderer::new();
    renderer.write_header(frame.size());
    renderer.render_frame(frame, Transform::identity());
    renderer.finalize()
}

/// Export multiple frames into a single SVG file.
///
/// The padding will be added around and between the individual frames.
#[tracing::instrument(skip_all)]
pub fn svg_merged(frames: &[Frame], padding: Abs) -> String {
    let width = 2.0 * padding
        + frames.iter().map(|frame| frame.width()).max().unwrap_or_default();
    let height = padding + frames.iter().map(|page| page.height() + padding).sum::<Abs>();
    let size = Size::new(width, height);

    let mut renderer = SVGRenderer::new();
    renderer.write_header(size);

    let [x, mut y] = [padding; 2];
    for frame in frames {
        renderer.render_frame(frame, Transform::translate(x, y));
        y += frame.height() + padding;
    }

    renderer.finalize()
}

/// Renders one or multiple frames to an SVG file.
struct SVGRenderer {
    /// The internal XML writer.
    xml: XmlWriter,
    /// Prepared glyphs.
    glyphs: Deduplicator<RenderedGlyph>,
    /// Clip paths are used to clip a group. A clip path is a path that defines
    /// the clipping region. The clip path is referenced by the `clip-path`
    /// attribute of the group. The clip path is in the format of `M x y L x y C
    /// x1 y1 x2 y2 x y Z`.
    clip_paths: Deduplicator<EcoString>,
}

/// Represents a glyph to be rendered.
enum RenderedGlyph {
    /// A path is a sequence of drawing commands.
    ///
    /// It is in the format of `M x y L x y C x1 y1 x2 y2 x y Z`.
    Path(EcoString),
    /// An image is a URL to an image file, plus the size and transform.
    ///
    /// The url is in the format of `data:image/{format};base64,`.
    Image { url: EcoString, width: f64, height: f64, ts: Transform },
}

impl SVGRenderer {
    /// Create a new SVG renderer with empty glyph and clip path.
    fn new() -> Self {
        SVGRenderer {
            xml: XmlWriter::new(xmlwriter::Options::default()),
            glyphs: Deduplicator::new('g'),
            clip_paths: Deduplicator::new('c'),
        }
    }

    /// Write the SVG header, including the `viewBox` and `width` and `height`
    /// attributes.
    fn write_header(&mut self, size: Size) {
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

    /// Render a frame with the given transform.
    fn render_frame(&mut self, frame: &Frame, ts: Transform) {
        self.xml.start_element("g");
        if !ts.is_identity() {
            self.xml.write_attribute("transform", &SvgMatrix(ts));
        }

        for (pos, item) in frame.items() {
            let x = pos.x.to_pt();
            let y = pos.y.to_pt();
            self.xml.start_element("g");
            self.xml
                .write_attribute_fmt("transform", format_args!("translate({x} {y})"));

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

    /// Render a group. If the group has `clips` set to true, a clip path will
    /// be created.
    fn render_group(&mut self, group: &GroupItem) {
        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-group");

        if group.clips {
            let hash = hash128(&group);
            let size = group.frame.size();
            let x = size.x.to_pt();
            let y = size.y.to_pt();
            let id = self.clip_paths.insert_with(hash, || {
                let mut builder = SvgPathBuilder(EcoString::new());
                builder.rect(x as f32, y as f32);
                builder.0
            });
            self.xml.write_attribute_fmt("clip-path", format_args!("url(#{id})"));
        }

        self.render_frame(&group.frame, group.transform);
        self.xml.end_element();
    }

    /// Render a text item. The text is rendered as a group of glyphs. We will
    /// try to render the text as SVG first, then bitmap, then outline. If none
    /// of them works, we will skip the text.
    fn render_text(&mut self, text: &TextItem) {
        let scale: f64 = text.size.to_pt() / text.font.units_per_em();
        let inv_scale: f64 = text.font.units_per_em() / text.size.to_pt();

        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-text");
        self.xml.write_attribute_fmt(
            "transform",
            format_args!("scale({} {})", scale, -scale),
        );

        let mut x: f64 = 0.0;
        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let offset = x + glyph.x_offset.at(text.size).to_pt();

            self.render_svg_glyph(text, id, offset, inv_scale)
                .or_else(|| self.render_bitmap_glyph(text, id, offset, inv_scale))
                .or_else(|| self.render_outline_glyph(text, id, offset, inv_scale));

            x += glyph.x_advance.at(text.size).to_pt();
        }

        self.xml.end_element();
    }

    /// Render a glyph defined by an SVG.
    fn render_svg_glyph(
        &mut self,
        text: &TextItem,
        id: GlyphId,
        x_offset: f64,
        inv_scale: f64,
    ) -> Option<()> {
        let data_url = convert_svg_glyph_to_base64_url(&text.font, id)?;
        let upem = Abs::raw(text.font.units_per_em());
        let origin_ascender = text.font.metrics().ascender.at(upem).to_pt();

        let glyph_hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(glyph_hash, || RenderedGlyph::Image {
            url: data_url,
            width: upem.to_pt(),
            height: upem.to_pt(),
            ts: Transform::translate(Abs::zero(), Abs::pt(-origin_ascender))
                .post_concat(Transform::scale(Ratio::new(1.0), Ratio::new(-1.0))),
        });

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml
            .write_attribute_fmt("x", format_args!("{}", x_offset * inv_scale));
        self.xml.end_element();

        Some(())
    }

    /// Render a glyph defined by a bitmap.
    fn render_bitmap_glyph(
        &mut self,
        text: &TextItem,
        id: GlyphId,
        x_offset: f64,
        inv_scale: f64,
    ) -> Option<()> {
        let (image, bitmap_x_offset, bitmap_y_offset) =
            convert_bitmap_glyph_to_image(&text.font, id)?;

        let glyph_hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(glyph_hash, || {
            let width = image.width() as f64;
            let height = image.height() as f64;
            let url = convert_image_to_base64_url(&image);
            let ts = Transform::translate(
                Abs::pt(bitmap_x_offset),
                Abs::pt(-height - bitmap_y_offset),
            );
            RenderedGlyph::Image { url, width, height, ts }
        });

        let target_height = text.size.to_pt();
        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));

        // The image is stored with the height of `image.height()`, but we want
        // to render it with a height of `target_height`. So we need to scale
        // it.
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

    /// Render a glyph defined by an outline.
    fn render_outline_glyph(
        &mut self,
        text: &TextItem,
        id: GlyphId,
        x_offset: f64,
        inv_scale: f64,
    ) -> Option<()> {
        let path = convert_outline_glyph_to_path(&text.font, id)?;
        let hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(hash, || RenderedGlyph::Path(path));

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml
            .write_attribute_fmt("x", format_args!("{}", x_offset * inv_scale));
        self.write_fill(&text.fill);
        self.xml.end_element();

        Some(())
    }

    /// Render a shape element.
    fn render_shape(&mut self, shape: &Shape) {
        self.xml.start_element("path");
        self.xml.write_attribute("class", "typst-shape");

        if let Some(paint) = &shape.fill {
            self.write_fill(paint);
        } else {
            self.xml.write_attribute("fill", "none");
        }

        if let Some(stroke) = &shape.stroke {
            self.write_stroke(stroke);
        }

        let path = convert_geometry_to_path(&shape.geometry);
        self.xml.write_attribute("d", &path);
        self.xml.end_element();
    }

    /// Write a fill attribute.
    fn write_fill(&mut self, fill: &Paint) {
        let Paint::Solid(color) = fill;
        self.xml.write_attribute("fill", &color.to_rgba().to_hex());
    }

    /// Write a stroke attribute.
    fn write_stroke(&mut self, stroke: &Stroke) {
        let Paint::Solid(color) = stroke.paint;
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

    /// Render an image element.
    fn render_image(&mut self, image: &Image, size: &Axes<Abs>) {
        let url = convert_image_to_base64_url(image);
        self.xml.start_element("image");
        self.xml.write_attribute("xlink:href", &url);
        self.xml.write_attribute("width", &size.x.to_pt());
        self.xml.write_attribute("height", &size.y.to_pt());
        self.xml.write_attribute("preserveAspectRatio", "none");
        self.xml.end_element();
    }

    /// Finalize the SVG file. This must be called after all rendering is done.
    fn finalize(mut self) -> String {
        self.write_glyph_defs();
        self.write_clip_path_defs();
        self.xml.end_document()
    }

    /// Build the glyph definitions.
    fn write_glyph_defs(&mut self) {
        self.xml.start_element("defs");
        self.xml.write_attribute("id", "glyph");

        for (id, glyph) in self.glyphs.iter() {
            self.xml.start_element("symbol");
            self.xml.write_attribute("id", &id);
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
                        self.xml.write_attribute("transform", &SvgMatrix(*ts));
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
    fn write_clip_path_defs(&mut self) {
        self.xml.start_element("defs");
        self.xml.write_attribute("id", "clip-path");

        for (id, path) in self.clip_paths.iter() {
            self.xml.start_element("clipPath");
            self.xml.write_attribute("id", &id);
            self.xml.start_element("path");
            self.xml.write_attribute("d", &path);
            self.xml.end_element();
            self.xml.end_element();
        }

        self.xml.end_element();
    }
}

/// Convert an outline glyph to an SVG path.
#[comemo::memoize]
fn convert_outline_glyph_to_path(font: &Font, id: GlyphId) -> Option<EcoString> {
    let mut builder = SvgPathBuilder::default();
    font.ttf().outline_glyph(id, &mut builder)?;
    Some(builder.0)
}

/// Convert a bitmap glyph to an encoded image URL.
#[comemo::memoize]
fn convert_bitmap_glyph_to_image(font: &Font, id: GlyphId) -> Option<(Image, f64, f64)> {
    let bitmap = font.ttf().glyph_raster_image(id, std::u16::MAX)?;
    let image = Image::new(bitmap.data.into(), bitmap.format.into(), None).ok()?;
    Some((image, bitmap.x as f64, bitmap.y as f64))
}

/// Convert an SVG glyph to an encoded image URL.
#[comemo::memoize]
fn convert_svg_glyph_to_base64_url(font: &Font, id: GlyphId) -> Option<EcoString> {
    let mut data = font.ttf().glyph_svg_image(id)?;

    // Decompress SVGZ.
    let mut decoded = vec![];
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
    let mut start_span = None;
    let mut last_viewbox = None;

    // Parse xml and find the viewBox of the svg element.
    // <svg viewBox="0 0 1000 1000">...</svg>
    // ~~~~~^~~~~~~
    for n in xmlparser::Tokenizer::from(svg_str.as_str()) {
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
        // Correct the viewbox if it is not present. `-origin_ascender` is to
        // make sure the glyph is rendered at the correct position
        svg_str.insert_str(
            start_span.unwrap().range().end,
            format!(r#" viewBox="0 {} {width} {height}""#, -origin_ascender).as_str(),
        );
    }

    let mut url: EcoString = "data:image/svg+xml;base64,".into();
    let b64_encoded =
        base64::engine::general_purpose::STANDARD.encode(svg_str.as_bytes());
    url.push_str(&b64_encoded);

    Some(url)
}

/// Convert a geometry to an SVG path.
#[comemo::memoize]
fn convert_geometry_to_path(geometry: &Geometry) -> EcoString {
    let mut builder = SvgPathBuilder::default();
    match geometry {
        Geometry::Line(t) => {
            builder.move_to(0.0, 0.0);
            builder.line_to(t.x.to_pt() as f32, t.y.to_pt() as f32);
        }
        Geometry::Rect(rect) => {
            let x = rect.x.to_pt() as f32;
            let y = rect.y.to_pt() as f32;
            builder.rect(x, y);
        }
        Geometry::Path(p) => {
            for item in &p.0 {
                match item {
                    PathItem::MoveTo(m) => {
                        builder.move_to(m.x.to_pt() as f32, m.y.to_pt() as f32)
                    }
                    PathItem::LineTo(l) => {
                        builder.line_to(l.x.to_pt() as f32, l.y.to_pt() as f32)
                    }
                    PathItem::CubicTo(c1, c2, t) => builder.curve_to(
                        c1.x.to_pt() as f32,
                        c1.y.to_pt() as f32,
                        c2.x.to_pt() as f32,
                        c2.y.to_pt() as f32,
                        t.x.to_pt() as f32,
                        t.y.to_pt() as f32,
                    ),
                    PathItem::ClosePath => builder.close(),
                }
            }
        }
    };
    builder.0
}

/// Encode an image into a data URL. The format of the URL is
/// `data:image/{format};base64,`.
#[comemo::memoize]
fn convert_image_to_base64_url(image: &Image) -> EcoString {
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

    let mut url = eco_format!("data:image/{format};base64,");
    let data = base64::engine::general_purpose::STANDARD.encode(image.data());
    url.push_str(&data);
    url
}

/// Deduplicates its elements. It is used to deduplicate glyphs and clip paths.
/// The `H` is the hash type, and `T` is the value type. The `PREFIX` is the
/// prefix of the index. This is used to distinguish between glyphs and clip
/// paths.
#[derive(Debug, Clone)]
struct Deduplicator<T> {
    kind: char,
    vec: Vec<T>,
    present: HashMap<u128, Id>,
}

impl<T> Deduplicator<T> {
    fn new(kind: char) -> Self {
        Self { kind, vec: Vec::new(), present: HashMap::new() }
    }

    /// Inserts a value into the vector. If the hash is already present, returns
    /// the index of the existing value and `f` will not be called. Otherwise,
    /// inserts the value and returns the id of the inserted value.
    #[must_use = "returns the index of the inserted value"]
    fn insert_with<F>(&mut self, hash: u128, f: F) -> Id
    where
        F: FnOnce() -> T,
    {
        *self.present.entry(hash).or_insert_with(|| {
            let index = self.vec.len();
            self.vec.push(f());
            Id(self.kind, index)
        })
    }

    /// Iterate over the the elements alongside their ids.
    fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.vec.iter().enumerate().map(|(i, v)| (Id(self.kind, i), v))
    }
}

/// Identifies a `<def>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Id(char, usize);

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

/// Displays as an SVG matrix.
struct SvgMatrix(Transform);

impl Display for SvgMatrix {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Convert a [`Transform`] into a SVG transform string.
        // See https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform
        write!(
            f,
            "matrix({} {} {} {} {} {})",
            self.0.sx.get(),
            self.0.ky.get(),
            self.0.kx.get(),
            self.0.sy.get(),
            self.0.tx.to_pt(),
            self.0.ty.to_pt()
        )
    }
}

/// A builder for SVG path.
#[derive(Default)]
struct SvgPathBuilder(pub EcoString);

impl SvgPathBuilder {
    /// Create a rectangle path. The rectangle is created with the top-left
    /// corner at (0, 0). The width and height are the size of the rectangle.
    fn rect(&mut self, width: f32, height: f32) {
        self.move_to(0.0, 0.0);
        self.line_to(0.0, height);
        self.line_to(width, height);
        self.line_to(width, 0.0);
        self.close();
    }
}

/// A builder for SVG path. This is used to build the path for a glyph.
impl ttf_parser::OutlineBuilder for SvgPathBuilder {
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

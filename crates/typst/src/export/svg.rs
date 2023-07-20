use std::{
    collections::HashMap,
    fmt::{Display, Write},
    io::Read,
};

use base64::Engine;
use ttf_parser::{GlyphId, OutlineBuilder};
use usvg::{NodeExt, TreeParsing};
use xmlwriter::XmlWriter;

use crate::{
    doc::{Document, Frame, FrameItem, Glyph, GroupItem, TextItem},
    geom::{Abs, Axes, Geometry, LineCap, LineJoin, Shape, Transform},
    image::{ImageFormat, RasterFormat, VectorFormat},
    util::hash128,
};
use crate::{geom::Paint::Solid, image::Image};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RenderHash(u128);

impl From<u128> for RenderHash {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl Display for RenderHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        base64::engine::general_purpose::STANDARD
            .encode(self.0.to_le_bytes())
            .fmt(f)
    }
}

/// Export a document into a SVG file.
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

enum RenderedGlyph {
    Path(String),
    Image { url: String, width: f64, height: f64, ts: Transform },
}

struct SVGRenderer {
    xml: XmlWriter,
    glyphs: HashMap<RenderHash, RenderedGlyph>,
    clip_paths: HashMap<RenderHash, String>,
}

impl SVGRenderer {
    fn new() -> Self {
        SVGRenderer {
            xml: XmlWriter::new(xmlwriter::Options::default()),
            glyphs: HashMap::default(),
            clip_paths: HashMap::default(),
        }
    }

    fn header(&mut self, size: Axes<Abs>) {
        self.xml.start_element("svg");
        self.xml.write_attribute("class", "typst-doc");
        self.xml.write_attribute_fmt(
            "viewBox",
            format_args!("0 0 {} {}", size.x.to_pt(), size.y.to_pt()),
        );
        self.xml.write_attribute("width", &size.x.to_pt().to_string());
        self.xml.write_attribute("height", &size.y.to_pt().to_string());
        self.xml.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        self.xml
            .write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        self.xml.write_attribute("xmlns:h5", "http://www.w3.org/1999/xhtml");
    }

    fn build_glyph(&mut self) {
        self.xml.start_element("defs");
        self.xml.write_attribute("id", "glyph");
        for (id, glyph) in &self.glyphs {
            self.xml.start_element("symbol");
            self.xml.write_attribute("id", &id.to_string());
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
                    self.xml.write_attribute("width", &width.to_string());
                    self.xml.write_attribute("height", &height.to_string());
                    if !ts.is_identity() {
                        self.xml.write_attribute("transform", &ts.to_svg());
                    }
                    self.xml.write_attribute("preserveAspectRatio", "none");
                    self.xml.end_element();
                }
            }
            self.xml.end_element();
        }
        self.xml.end_element();
    }

    fn build_clip_path(&mut self) {
        self.xml.start_element("defs");
        self.xml.write_attribute("id", "clip-path");
        for (id, path) in &self.clip_paths {
            self.xml.start_element("clipPath");
            self.xml.write_attribute("id", &id.to_string());
            self.xml.start_element("path");
            self.xml.write_attribute("d", &path);
            self.xml.end_element();
            self.xml.end_element();
        }
        self.xml.end_element();
    }

    fn finalize(mut self) -> String {
        self.build_clip_path();
        self.build_glyph();
        self.xml.end_document()
    }

    fn render_frame(&mut self, frame: &Frame, trans: Transform) {
        self.xml.start_element("g");
        if !trans.is_identity() {
            self.xml.write_attribute("transform", &trans.to_svg());
        };
        for (pos, item) in frame.items() {
            let x = pos.x.to_pt();
            let y = pos.y.to_pt();
            self.xml.start_element("g");
            self.xml
                .write_attribute("transform", format!("translate({} {})", x, y).as_str());
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

    fn render_group(&mut self, group: &GroupItem) {
        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-group");
        if group.clips {
            let clip_path_hash = hash128(&group).into();
            let x = group.frame.size().x.to_pt();
            let y = group.frame.size().y.to_pt();
            self.clip_paths
                .entry(clip_path_hash)
                .or_insert_with(|| SVGPath2DBuilder::rect(x as f32, y as f32));
            self.xml.write_attribute_fmt(
                "clip-path",
                format_args!("url(#{})", clip_path_hash),
            );
        }
        self.render_frame(&group.frame, group.transform);
        self.xml.end_element();
    }

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
            self.render_svg_glyph(text, glyph, x_offset, inv_scale)
                .or_else(|| self.render_bitmap_glyph(text, glyph, x_offset, inv_scale))
                .or_else(|| self.render_outline_glyph(text, glyph, x_offset, inv_scale));
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
        let mut data = text.font.ttf().glyph_svg_image(GlyphId(glyph.id))?;
        let glyph_hash: RenderHash = hash128(&(&text.font, glyph.id)).into();

        // Decompress SVGZ.
        let mut decoded = vec![];
        if data.starts_with(&[0x1f, 0x8b]) {
            let mut decoder = flate2::read::GzDecoder::new(data);
            decoder.read_to_end(&mut decoded).ok()?;
            data = &decoded;
        }

        // Parse XML.
        let xml = std::str::from_utf8(data).ok()?;
        let document = roxmltree::Document::parse(xml).ok()?;

        // Parse SVG.
        let opts = usvg::Options::default();
        let tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;

        let size = text.size.to_pt();

        // Compute the space we need to draw our glyph.
        // See https://github.com/RazrFalcon/resvg/issues/602 for why
        // using the svg size is problematic here.
        let mut bbox = usvg::Rect::new_bbox();
        for node in tree.root.descendants() {
            if let Some(rect) = node.calculate_bbox().and_then(|b| b.to_rect()) {
                bbox = bbox.expand(rect);
            }
        }
        let height = size;
        let width = bbox.width() / bbox.height() * height;
        self.glyphs.entry(glyph_hash).or_insert_with(|| {
            let mut url = "data:image/svg+xml;base64,".to_string();
            // fixme: this is a hack to remove the viewbox from the glyph
            // this is because the viewbox of noto color emoji is wrong,
            let re = regex::Regex::new(r#"viewBox=".*?""#).unwrap();
            let xml = re.replace(xml, "");
            let data = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
            url.push_str(&data);
            RenderedGlyph::Image {
                url,
                width: width * inv_scale,
                height: height * inv_scale,
                ts: Transform::identity(),
            }
        });

        self.xml.start_element("use");
        self.xml
            .write_attribute_fmt("xlink:href", format_args!("#{}", glyph_hash));
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
        let bitmap =
            text.font.ttf().glyph_raster_image(GlyphId(glyph.id), std::u16::MAX)?;
        let glyph_hash: RenderHash = hash128(&(&text.font, glyph.id)).into();
        let image = Image::new(bitmap.data.into(), bitmap.format.into(), None).ok()?;
        self.glyphs.entry(glyph_hash).or_insert_with(|| {
            let width = image.width() as f64;
            let height = image.height() as f64;
            let x_offset = bitmap.x as f64;
            let y_offset = bitmap.y as f64;
            let url = encode_image_to_url(&image);
            let ts = Transform::translate(Abs::pt(x_offset), Abs::pt(-height - y_offset));
            RenderedGlyph::Image { url, width, height, ts }
        });
        let target_height = text.size.to_pt();
        self.xml.start_element("use");
        self.xml.write_attribute_fmt(
            "xlink:href",
            format_args!("#{}", &glyph_hash.to_string()),
        );
        self.xml.write_attribute("x", &(x_offset * inv_scale).to_string());
        self.xml.write_attribute_fmt(
            "transform",
            format_args!(
                "scale({} -{})",
                inv_scale * (target_height / image.height() as f64),
                inv_scale * (target_height / image.height() as f64),
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
        let mut builder = SVGPath2DBuilder(String::new());
        text.font.ttf().outline_glyph(GlyphId(glyph.id), &mut builder)?;
        let glyph_hash = hash128(&(&text.font, glyph.id)).into();
        self.glyphs.entry(glyph_hash).or_insert_with(|| {
            let path = builder.0;
            RenderedGlyph::Path(path)
        });
        let Solid(text_color) = text.fill;
        self.xml.start_element("use");
        self.xml
            .write_attribute_fmt("xlink:href", format_args!("#{}", glyph_hash));
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
            self.xml.write_attribute("stoke", &color.to_rgba().to_hex());
            self.xml
                .write_attribute("stroke-width", &stroke.thickness.to_pt().to_string());
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
            self.xml
                .write_attribute("stoke-miterlimit", &stroke.miter_limit.0.to_string());
            if let Some(pattern) = &stroke.dash_pattern {
                self.xml.write_attribute(
                    "stoken-dashoffset",
                    &pattern.phase.to_pt().to_string(),
                );
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
        let mut path_builder = SVGPath2DBuilder(String::new());
        match &shape.geometry {
            Geometry::Line(t) => {
                path_builder.move_to(0.0, 0.0);
                path_builder.line_to(t.x.to_pt() as f32, t.y.to_pt() as f32);
            }
            Geometry::Rect(rect) => {
                let x = rect.x.to_pt() as f32;
                let y = rect.y.to_pt() as f32;
                SVGPath2DBuilder::rect(x, y);
            }
            Geometry::Path(p) => {
                for item in &p.0 {
                    match item {
                        crate::geom::PathItem::MoveTo(m) => {
                            path_builder.move_to(m.x.to_pt() as f32, m.y.to_pt() as f32)
                        }
                        crate::geom::PathItem::LineTo(l) => {
                            path_builder.line_to(l.x.to_pt() as f32, l.y.to_pt() as f32)
                        }
                        crate::geom::PathItem::CubicTo(c1, c2, t) => path_builder
                            .curve_to(
                                c1.x.to_pt() as f32,
                                c1.y.to_pt() as f32,
                                c2.x.to_pt() as f32,
                                c2.y.to_pt() as f32,
                                t.x.to_pt() as f32,
                                t.y.to_pt() as f32,
                            ),
                        crate::geom::PathItem::ClosePath => path_builder.close(),
                    }
                }
            }
        };
        self.xml.write_attribute("d", &path_builder.0);
        self.xml.end_element();
    }

    fn render_image(&mut self, image: &Image, size: &Axes<Abs>) {
        let url = encode_image_to_url(image);
        self.xml.start_element("image");
        self.xml.write_attribute("xlink:href", &url);
        self.xml.write_attribute("width", &size.x.to_pt().to_string());
        self.xml.write_attribute("height", &size.y.to_pt().to_string());
        self.xml.write_attribute("preserveAspectRatio", "none");
        self.xml.end_element();
    }
}

fn encode_image_to_url(image: &Image) -> String {
    let format = match image.format() {
        ImageFormat::Raster(f) => match f {
            RasterFormat::Png => "jpeg",
            RasterFormat::Jpg => "png",
            RasterFormat::Gif => "gif",
        },
        ImageFormat::Vector(f) => match f {
            VectorFormat::Svg => "svg+xml",
        },
    };
    let mut url = format!("data:image/{};base64,", format);
    let data = base64::engine::general_purpose::STANDARD.encode(image.data());
    url.push_str(&data);
    url
}

trait TransformExt {
    fn to_svg(self) -> String;
}

impl TransformExt for Transform {
    fn to_svg(self) -> String {
        format!(
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

struct SVGPath2DBuilder(pub String);

impl SVGPath2DBuilder {
    fn rect(width: f32, height: f32) -> String {
        let mut builder = SVGPath2DBuilder(String::new());
        builder.move_to(0.0, 0.0);
        builder.line_to(0.0, height);
        builder.line_to(width, height);
        builder.line_to(width, 0.0);
        builder.close();
        builder.0
    }
}

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

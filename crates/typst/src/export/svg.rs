use std::{
    collections::HashMap,
    fmt::{Display, Write},
    io::Read,
};

use base64::Engine;
use ttf_parser::{GlyphId, OutlineBuilder};
use usvg::{NodeExt, TreeParsing};

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
    let mut renderer = SVGRenderer::default();
    let mut max_width = Abs::zero();
    let mut y_offset = Abs::zero();
    for page in &doc.pages {
        let page_string =
            renderer.render_frame(page, Transform::translate(Abs::zero(), y_offset));
        renderer.append_page(page_string);
        y_offset += page.size().y;
        max_width = max_width.max(page.size().x);
    }
    let doc_size = Axes { x: max_width, y: y_offset };
    renderer.finalize(doc_size)
}

#[derive(Debug, Clone, Default)]
struct SVGRenderer {
    body: String,
    glyphs: HashMap<RenderHash, String>,
    clip_paths: HashMap<RenderHash, String>,
}

impl SVGRenderer {
    fn header(&self, size: Axes<Abs>) -> String {
        let mut res = format!(
            r#"<svg class="typst-doc" viewBox="0 0 {0} {1}" width="{0}" height="{1}"
    xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
    xmlns:h5="http://www.w3.org/1999/xhtml">"#,
            size.x.to_pt(),
            size.y.to_pt()
        );
        res.push_str(r#"<defs id="glyph">"#);
        for path in self.glyphs.values() {
            res.push_str(path);
            res.push('\n');
        }
        res.push_str(r#"</defs>"#);
        res.push('\n');
        res.push_str(r#"<defs id="clip-path">"#);
        for (hash, path) in &self.clip_paths {
            res.push_str(
                format!(r#"<clipPath id="{}"> <path d="{}"/> </clipPath>"#, hash, path)
                    .as_str(),
            );
            res.push('\n');
        }
        res.push_str(r#"</defs>"#);
        res
    }

    fn finalize(&self, size: Axes<Abs>) -> String {
        let mut header = self.header(size);
        header.push_str(&self.body);
        header.push_str("</svg>");
        header
    }

    fn render_frame(&mut self, frame: &Frame, trans: Transform) -> String {
        let mut page = if trans.is_identity() {
            r#"<g>"#.to_string()
        } else {
            format!(r#"<g transform={}>"#, trans.to_svg())
        };
        for (pos, item) in frame.items() {
            let x = pos.x.to_f32();
            let y = pos.y.to_f32();
            let str = match item {
                FrameItem::Group(group) => self.render_group(group),
                FrameItem::Text(text) => self.render_text(text),
                FrameItem::Shape(shape, _) => self.render_shape(shape),
                FrameItem::Image(image, size, _) => self.render_image(image, size),
                FrameItem::Meta(_, _) => continue,
            };
            page.push_str(format!(r#"<g transform="translate({} {})">"#, x, y).as_str());
            page.push_str(&str);
            page.push('\n');
            page.push_str("</g>");
        }
        page.push_str("</g>");
        page
    }

    fn append_page(&mut self, page: String) {
        self.body.push_str(&page);
    }

    fn render_group(&mut self, group: &GroupItem) -> String {
        let mut str: String = String::new();
        if group.clips {
            let clip_path_hash = hash128(&group).into();
            let x = group.frame.size().x.to_f32();
            let y = group.frame.size().y.to_f32();
            self.clip_paths.entry(clip_path_hash).or_insert_with(|| {
                let mut builder = SVGPath2DBuilder(String::new());
                builder.move_to(0.0, 0.0);
                builder.line_to(0.0, y);
                builder.line_to(x, y);
                builder.line_to(x, 0.0);
                builder.close();
                builder.0
            });
            let clip = format!(r##"<g clip-path="url(#{})">"##, clip_path_hash);
            str.push_str(&clip);
        }
        let page = self.render_frame(&group.frame, group.transform);
        str.push_str(&page);
        if group.clips {
            str.push_str("</g>");
        }
        str
    }

    fn render_text(&mut self, text: &TextItem) -> String {
        let scale: f32 = (text.size.to_pt() / text.font.units_per_em()) as f32;
        let inv_scale: f32 = (text.font.units_per_em() / text.size.to_pt()) as f32;
        let mut res =
            format!(r#"<g class="typst-text" transform="scale({} {})">"#, scale, -scale);
        let mut x_offset: f32 = 0.0;
        for glyph in &text.glyphs {
            if let Some(rendered_glyph) = self
                .render_svg_glyph(text, glyph, x_offset, inv_scale)
                .or_else(|| self.render_bitmap_glyph(text, glyph, x_offset, inv_scale))
                .or_else(|| self.render_outline_glyph(text, glyph, x_offset, inv_scale))
            {
                res.push_str(&rendered_glyph);
            }
            x_offset += glyph.x_advance.at(text.size).to_f32();
        }
        res.push_str("</g>");
        res
    }

    fn render_svg_glyph(
        &mut self,
        text: &TextItem,
        glyph: &Glyph,
        x_offset: f32,
        inv_scale: f32,
    ) -> Option<String> {
        let mut data = text.font.ttf().glyph_svg_image(GlyphId(glyph.id))?;
        let glyph_hash: RenderHash = hash128(&(&text.font, glyph)).into();

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

        let size = text.size.to_f32();

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
        let width = (bbox.width() / bbox.height()) as f32 * height;
        self.glyphs.entry(glyph_hash).or_insert_with(|| {
            let mut url = "data:image/svg+xml;base64,".to_string();
            // fixme: this is a hack to remove the viewbox from the glyph
            // this is because the viewbox of noto color emoji is wrong,
            let re = regex::Regex::new(r#"viewBox=".*?""#).unwrap();
            let xml = re.replace(xml, "");
            let data = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
            url.push_str(&data);
            format!(
                r#"<symbol id="{glyph_hash}"><image xlink:href="{}" x="0" y="0" width="{}" height="{}" /></symbol>"#,
                url,
                width * inv_scale,
                height * inv_scale
            )
        });

        Some(format!(
            r##"<use xlink:href="#{}" x="{}"/>"##,
            glyph_hash,
            x_offset * inv_scale,
        ))
    }

    fn render_bitmap_glyph(
        &mut self,
        text: &TextItem,
        glyph: &Glyph,
        x_offset: f32,
        inv_scale: f32,
    ) -> Option<String> {
        let bitmap =
            text.font.ttf().glyph_raster_image(GlyphId(glyph.id), std::u16::MAX)?;
        let image = Image::new(bitmap.data.into(), bitmap.format.into(), None).ok()?;
        let size = text.size.to_f32();
        let h = text.size;
        let w = (image.width() as f64 / image.height() as f64) * h;
        let dx = (bitmap.x as f32) / (image.width() as f32) * size;
        let dy = (bitmap.y as f32) / (image.height() as f32) * size;

        let image = self.render_image(&image, &Axes { x: w, y: h });
        Some(format!(
            r#"<g transform="scale({inv_scale} -{inv_scale}) translate({}, {})"> {image} </g>"#,
            dx + x_offset,
            -size - dy,
        ))
    }

    fn render_outline_glyph(
        &mut self,
        text: &TextItem,
        glyph: &Glyph,
        x_offset: f32,
        inv_scale: f32,
    ) -> Option<String> {
        let mut builder = SVGPath2DBuilder(String::new());
        text.font.ttf().outline_glyph(GlyphId(glyph.id), &mut builder)?;
        let glyph_hash = hash128(&(&text.font, glyph)).into();
        self.glyphs.entry(glyph_hash).or_insert_with(|| {
            let path = builder.0;
            format!(
                r#"<symbol id="{}" overflow="visible"> <path d="{}"/> </symbol>"#,
                glyph_hash, path
            )
        });
        let Solid(text_color) = text.fill;

        Some(format!(
            r##"<use xlink:href="#{}" x="{}" fill="{}"/>"##,
            glyph_hash,
            x_offset * inv_scale,
            text_color.to_rgba().to_hex()
        ))
    }
    fn render_shape(&mut self, shape: &Shape) -> String {
        let mut attr_set = AttributeSet::default();
        if let Some(paint) = &shape.fill {
            let Solid(color) = paint;
            attr_set.set("fill", color.to_rgba().to_hex().to_string());
        } else {
            attr_set.set("fill", "none".to_string());
        }
        if let Some(stroke) = &shape.stroke {
            let Solid(color) = stroke.paint;
            attr_set.set("stroke", color.to_rgba().to_hex().to_string());
            attr_set.set("stroke-width", stroke.thickness.to_pt().to_string());
            attr_set.set(
                "stroke-linecap",
                match stroke.line_cap {
                    LineCap::Butt => "butt",
                    LineCap::Round => "round",
                    LineCap::Square => "square",
                }
                .to_string(),
            );
            attr_set.set(
                "stroke-linejoin",
                match stroke.line_join {
                    LineJoin::Miter => "miter",
                    LineJoin::Round => "round",
                    LineJoin::Bevel => "bevel",
                }
                .to_string(),
            );
            attr_set.set("stroke-miterlimit", stroke.miter_limit.0.to_string());
            if let Some(pattern) = &stroke.dash_pattern {
                attr_set.set("stroke-dashoffset", pattern.phase.to_pt().to_string());
                attr_set.set(
                    "stroke-dasharray",
                    pattern
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
                path_builder.line_to(t.x.to_f32(), t.y.to_f32());
            }
            Geometry::Rect(rect) => {
                let x = rect.x.to_f32();
                let y = rect.y.to_f32();
                // 0,0 <-> x,y
                path_builder.move_to(0.0, 0.0);
                path_builder.line_to(0.0, y);
                path_builder.line_to(x, y);
                path_builder.line_to(x, 0.0);
                path_builder.close();
            }
            Geometry::Path(p) => {
                for item in &p.0 {
                    match item {
                        crate::geom::PathItem::MoveTo(m) => {
                            path_builder.move_to(m.x.to_f32(), m.y.to_f32())
                        }
                        crate::geom::PathItem::LineTo(l) => {
                            path_builder.line_to(l.x.to_f32(), l.y.to_f32())
                        }
                        crate::geom::PathItem::CubicTo(c1, c2, t) => path_builder
                            .curve_to(
                                c1.x.to_f32(),
                                c1.y.to_f32(),
                                c2.x.to_f32(),
                                c2.y.to_f32(),
                                t.x.to_f32(),
                                t.y.to_f32(),
                            ),
                        crate::geom::PathItem::ClosePath => path_builder.close(),
                    }
                }
            }
        };
        format!(r#"<path d="{}" {} />"#, path_builder.0, attr_set,)
    }

    fn render_image(&mut self, image: &Image, size: &Axes<Abs>) -> String {
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
        format!(
            r#"<image x="0" y="0" width="{}" height="{}" xlink:href="{}" preserveAspectRatio="none" />"#,
            size.x.to_pt(),
            size.y.to_pt(),
            url
        )
    }
}

/// Additional methods for [`Length`].
trait AbsExt {
    /// Convert to a number of points as f32.
    fn to_f32(self) -> f32;
}

impl AbsExt for Abs {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

trait TransformExt {
    fn to_svg(self) -> String;
}

impl TransformExt for Transform {
    fn to_svg(self) -> String {
        format!(
            "\"matrix({} {} {} {} {} {})\"",
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct AttributeSet(HashMap<String, String>);

impl Display for AttributeSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, value) in &self.0 {
            write!(f, r#" {}="{}""#, key, value)?;
        }
        Ok(())
    }
}

impl AttributeSet {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

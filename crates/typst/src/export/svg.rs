use std::{
    collections::HashMap,
    fmt::{Display, Write},
};

use base64::Engine;
use ttf_parser::{GlyphId, OutlineBuilder};

use crate::{
    doc::{Frame, TextItem},
    geom::{Abs, Axes, Shape, Transform},
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
pub fn svg(page: &Frame) -> String {
    let mut renderer = SVGRenderer::default();
    let page_string = renderer.render_page(page, Transform::identity());
    renderer.append_page(page_string);
    renderer.finalize(page.size())
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
        for (hash, path) in &self.glyphs {
            res.push_str(
                format!(
                    r#"<symbol id="{}" overflow="visible"> <path d="{}"/> </symbol>"#,
                    hash, path
                )
                .as_str(),
            );
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

    fn render_page(&mut self, frame: &Frame, trans: Transform) -> String {
        let mut page = if trans.is_identity() {
            r#"<g>"#.to_string()
        } else {
            format!(r#"<g transform={}>"#, trans.to_svg())
        };
        for (pos, item) in frame.items() {
            let x = pos.x.to_f32();
            let y = pos.y.to_f32();
            let str = match item {
                crate::doc::FrameItem::Group(group) => {
                    let mut str = String::new();
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
                        let clip =
                            format!(r##"<g clip-path="url(#{})">"##, clip_path_hash);
                        str.push_str(&clip);
                    }
                    let page = self.render_page(&group.frame, group.transform);
                    str.push_str(&page);
                    if group.clips {
                        str.push_str("</g>");
                    }
                    str
                }
                crate::doc::FrameItem::Text(text) => self.render_text(text),
                crate::doc::FrameItem::Shape(shape, _) => self.render_shape(shape),
                crate::doc::FrameItem::Image(image, size, _) => {
                    self.render_image(image, size)
                }
                crate::doc::FrameItem::Meta(_, _) => continue,
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

    fn render_text(&mut self, text: &TextItem) -> String {
        let scale: f32 = (text.size.to_pt() / text.font.units_per_em()) as f32;
        let inv_scale: f32 = (text.font.units_per_em() / text.size.to_pt()) as f32;
        let mut res =
            format!(r#"<g class="typst-text" transform="scale({} {})">"#, scale, -scale);
        let mut x_offset: f32 = 0.0;
        for glyph in &text.glyphs {
            let glyph_hash = hash128(&(&text.font, glyph)).into();
            // fixme: only outline glyph for now
            self.glyphs.entry(glyph_hash).or_insert_with(|| {
                let mut builder = SVGPath2DBuilder(String::new());
                text.font.ttf().outline_glyph(GlyphId(glyph.id), &mut builder);
                builder.0
            });
            let Solid(text_color) = text.fill;
            res.push_str(
                format!(
                    r##"<use xlink:href="#{}" x="{}" fill="{}"/>"##,
                    glyph_hash,
                    x_offset * inv_scale,
                    text_color.to_rgba().to_hex()
                )
                .as_str(),
            );
            x_offset += glyph.x_advance.at(text.size).to_f32();
        }
        res.push_str("</g>");
        res
    }

    fn render_shape(&mut self, shape: &Shape) -> String {
        let mut attr_set = AttributeSet::default();
        if let Some(paint) = &shape.fill {
            let Solid(color) = paint;
            attr_set.set("fill", color.to_rgba().to_hex().to_string());
        }
        if let Some(stroke) = &shape.stroke {
            let Solid(color) = stroke.paint;
            attr_set.set("stroke", color.to_rgba().to_hex().to_string());
            attr_set.set("stroke-width", stroke.thickness.to_pt().to_string());
            attr_set.set(
                "stroke-linecap",
                match stroke.line_cap {
                    crate::geom::LineCap::Butt => "butt",
                    crate::geom::LineCap::Round => "round",
                    crate::geom::LineCap::Square => "square",
                }
                .to_string(),
            );
            attr_set.set(
                "stroke-linejoin",
                match stroke.line_join {
                    crate::geom::LineJoin::Miter => "miter",
                    crate::geom::LineJoin::Round => "round",
                    crate::geom::LineJoin::Bevel => "bevel",
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
            crate::geom::Geometry::Line(t) => {
                path_builder.move_to(0.0, 0.0);
                path_builder.line_to(t.x.to_f32(), t.y.to_f32());
            }
            crate::geom::Geometry::Rect(rect) => {
                let x = rect.x.to_f32();
                let y = rect.y.to_f32();
                // 0,0 <-> x,y
                path_builder.move_to(0.0, 0.0);
                path_builder.line_to(0.0, y);
                path_builder.line_to(x, y);
                path_builder.line_to(x, 0.0);
                path_builder.close();
            }
            crate::geom::Geometry::Path(p) => {
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
            crate::image::ImageFormat::Raster(f) => match f {
                crate::image::RasterFormat::Png => "jpeg",
                crate::image::RasterFormat::Jpg => "png",
                crate::image::RasterFormat::Gif => "gif",
            },
            crate::image::ImageFormat::Vector(f) => match f {
                crate::image::VectorFormat::Svg => "svg+xml",
            },
        };
        let mut url = format!("data:image/{};base64,", format);
        let data = base64::engine::general_purpose::STANDARD.encode(image.data());
        url.push_str(&data);
        format!(
            r#"<image x="0" y="0" width="{}" height="{}" style="fill" xlink:href="{}" preserveAspectRatio="none" />"#,
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

    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }
}

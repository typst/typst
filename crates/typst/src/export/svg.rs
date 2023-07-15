use std::{
    collections::HashMap,
    fmt::{Display, Write},
};

use ecow::EcoString;
use ttf_parser::{GlyphId, OutlineBuilder};

use crate::{
    doc::{Document, Frame, Meta, TextItem},
    font::Font,
    geom::{Abs, Axes, Transform},
    util::hash128,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphHash(u128);

impl From<u128> for GlyphHash {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl Display for GlyphHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
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
    glyphs: HashMap<GlyphHash, String>,
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
                    assert!(!group.clips); // fixme: assume that group has no clip path
                    self.render_page(&group.frame, group.transform)
                }
                crate::doc::FrameItem::Text(text) => self.render_text(text),
                crate::doc::FrameItem::Shape(_, _) => todo!(),
                crate::doc::FrameItem::Image(_, _, _) => todo!(),
                crate::doc::FrameItem::Meta(_, _) => continue,
            };
            page.push_str(format!(r#"<g transform="translate({} {})">"#, x, y).as_str());
            page.push_str(&str);
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
                let mut builder = OutlineGlyphBuilder(String::new());
                text.font.ttf().outline_glyph(GlyphId(glyph.id), &mut builder);
                builder.0
            });
            res.push_str(
                format!(
                    r##"<use xlink:href="#{}" x="{}"/>"##,
                    glyph_hash,
                    x_offset * inv_scale
                )
                .as_str(),
            );
            x_offset += glyph.x_advance.at(text.size).to_f32();
        }
        res.push_str("</g>");
        res
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

struct OutlineGlyphBuilder(pub String);

impl ttf_parser::OutlineBuilder for OutlineGlyphBuilder {
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

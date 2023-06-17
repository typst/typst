use std::sync::Arc;

use base64::Engine;

use super::{
    ir::{
        self, Abs, AbsoulteRef, Axes, Image, PathItem, PathStyle, Ratio, Scalar, Size,
        StyleNs,
    },
    GroupContext, RenderVm, TransformContext,
};
use crate::export::svg::{
    escape::{self, TextContentDataEscapes},
    ExportFeature, SvgRenderTask,
};

/// A generated text content.
pub enum SvgText {
    /// Append a plain text.
    Plain(String),
    /// Append a SVG/XML text node.
    Content(Arc<SvgTextNode>),
}

impl SvgText {
    /// Recursively estimate the length of the text node for final string allocation.
    pub fn estimated_len(&self) -> usize {
        match self {
            Self::Plain(p) => p.len(),
            Self::Content(c) => c.estimated_len(),
        }
    }

    /// Recursively write the text content to the string.
    pub fn write_string_io(&self, string_io: &mut String) {
        match self {
            SvgText::Plain(c) => string_io.push_str(c),
            SvgText::Content(c) => c.write_string_io(string_io),
        }
    }
}

impl From<&str> for SvgText {
    fn from(s: &str) -> Self {
        SvgText::Plain(s.to_string())
    }
}

/// A generated text node in SVG/XML format.
pub struct SvgTextNode {
    pub attributes: Vec<(&'static str, String)>,
    pub content: Vec<SvgText>,
}

impl SvgTextNode {
    /// Recursively estimate the length of the text node for final string allocation.
    pub fn estimated_len(&self) -> usize {
        let content_estimated: usize =
            self.content.iter().map(SvgText::estimated_len).sum();
        let attr_estimated: usize =
            self.attributes.iter().map(|attr| attr.0.len() + attr.1.len()).sum();

        "<g>".len()
            + (r#" ="""#.len() * self.attributes.len() + attr_estimated)
            + content_estimated
            + "</g>".len()
    }

    /// Recursively write the text content to the string.
    pub fn write_string_io(&self, string_io: &mut String) {
        string_io.push_str("<g");
        for (attr_name, attr_content) in &self.attributes {
            string_io.push(' ');
            string_io.push_str(attr_name);
            string_io.push('=');
            string_io.push('"');
            string_io.push_str(attr_content);
            string_io.push('"');
        }
        string_io.push('>');
        for c in &self.content {
            c.write_string_io(string_io)
        }
        string_io.push_str("</g>");
    }
}

/// A builder for [`SvgTextNode`].
/// It holds a reference to [`SvgRenderTask`] and state of the building process.
pub struct SvgTextBuilder<'s, 'm, 't, Feat: ExportFeature> {
    pub t: &'s mut SvgRenderTask<'m, 't, Feat>,
    pub attributes: Vec<(&'static str, String)>,
    pub content: Vec<SvgText>,
}

impl<'s, 'm, 't, Feat: ExportFeature> From<SvgTextBuilder<'s, 'm, 't, Feat>>
    for Arc<SvgTextNode>
{
    fn from(s: SvgTextBuilder<'s, 'm, 't, Feat>) -> Self {
        Arc::new(SvgTextNode { attributes: s.attributes, content: s.content })
    }
}

/// Internal methods for [`SvgTextBuilder`].
impl<'s, 'm, 't, Feat: ExportFeature> SvgTextBuilder<'s, 'm, 't, Feat> {
    pub fn with_text_shape(&mut self, shape: &ir::TextShape) {
        let fill = if shape.fill.as_ref() == "#000" {
            r#"tb"#.to_owned()
        } else {
            let fill_id = format!(r#"f{}"#, shape.fill.trim_start_matches('#'));
            let fill_key = (StyleNs::Fill, shape.fill.clone());
            self.t.style_defs.entry(fill_key).or_insert_with(|| {
                format!(r#"g.{} {{ --glyph_fill: {}; }} "#, fill_id, shape.fill)
            });

            fill_id
        };

        self.attributes.push(("class", format!("typst-txt {}", fill)));
    }

    pub fn render_glyph_ref_inner(&mut self, pos: Scalar, glyph: &AbsoulteRef) {
        let adjusted = (pos.0 * 2.).round() / 2.;

        let glyph_id = if Feat::USE_STABLE_GLYPH_ID && self.t.use_stable_glyph_id {
            glyph.as_svg_id("g")
        } else {
            glyph.as_unstable_svg_id("g")
        };
        let e = format!(r##"<use style="--o: {}" href="#{}"/>"##, adjusted, glyph_id);

        self.content.push(SvgText::Plain(e));
    }

    pub fn render_text_semantics_inner(
        &mut self,
        shape: &ir::TextShape,
        content: &str,
        width: Scalar,
    ) {
        // Scale is in pixel per em, but curve data is in font design units, so
        // we have to divide by units per em.
        let upem = shape.upem.0;
        let ppem = shape.ppem.0;
        let ascender = shape.ascender.0;

        let width = width.0 / ppem;
        #[cfg(feature = "fg_text_layout")]
        if false {
            let mut text_content = String::new();
            if !text.content.content.is_empty() {
                let per_width = width / text.content.content.len() as f32;
                for (i, c) in text.content.content.chars().enumerate() {
                    text_content.push_str(&format!(
                        r#"<span style="left: {}px">"#,
                        i as f32 * per_width,
                    ));
                    match c {
                        '<' => text_content.push_str("&lt;"),
                        '&' => text_content.push_str("&amp;"),
                        ' ' => text_content.push_str("&nbsp;"),
                        _ => text_content.push(c),
                    }
                    text_content.push_str(r#"</span>"#);
                }
            }
        }
        let text_content = escape::escape_str::<TextContentDataEscapes>(content);

        // todo: investigate &nbsp;
        let text_content = format!(
            r#"<g transform="scale(1,-1)"><foreignObject x="0" y="-{}" width="{}" height="{}"><h5:div class="tsel" style="font-size: {}px">{}</h5:div></foreignObject></g>"#,
            ascender / ppem,
            width,
            upem,
            upem,
            text_content
        );

        self.content.push(SvgText::Plain(text_content))
    }
}

/// See [`TransformContext`].
impl<'s, 'm, 't, Feat: ExportFeature> TransformContext
    for SvgTextBuilder<'s, 'm, 't, Feat>
{
    fn transform_matrix(mut self, m: &ir::Transform) -> Self {
        self.attributes.push((
            "transform",
            format!(
                r#"matrix({},{},{},{},{},{})"#,
                m.sx.0, m.ky.0, m.kx.0, m.sy.0, m.tx.0, m.ty.0
            ),
        ));
        self
    }

    fn transform_translate(mut self, matrix: Axes<Abs>) -> Self {
        self.attributes.push((
            "transform",
            format!(r#"translate({:.3},{:.3})"#, matrix.x.0, matrix.y.0),
        ));
        self
    }

    fn transform_scale(mut self, x: Ratio, y: Ratio) -> Self {
        self.attributes
            .push(("transform", format!(r#"scale({},{})"#, x.0, y.0)));
        self
    }

    fn transform_rotate(mut self, matrix: Scalar) -> Self {
        self.attributes
            .push(("transform", format!(r#"rotate({})"#, matrix.0)));
        self
    }

    fn transform_skew(mut self, matrix: (Ratio, Ratio)) -> Self {
        self.attributes.push((
            "transform",
            format!(r#"skewX({}) skewY({})"#, matrix.0 .0, matrix.1 .0),
        ));
        self
    }

    fn transform_clip(mut self, matrix: &ir::PathItem) -> Self {
        let clip_id;
        if let Some(c) = self.t.clip_paths.get(&matrix.d) {
            clip_id = *c;
        } else {
            let cid = self.t.clip_paths.len() as u32;
            self.t.clip_paths.insert(matrix.d.clone(), cid);
            clip_id = cid;
        }

        self.attributes
            .push(("clip-path", format!(r##"url(#c{:x})"##, clip_id)));
        self
    }
}

/// See [`GroupContext`].
impl<'s, 'm, 't, Feat: ExportFeature> GroupContext for SvgTextBuilder<'s, 'm, 't, Feat> {
    fn render_item_at(&mut self, pos: ir::Point, item: &ir::SvgItem) {
        self.content.push(SvgText::Content(Arc::new(SvgTextNode {
            attributes: vec![(
                "transform",
                format!("translate({:.3},{:.3})", pos.x.0, pos.y.0),
            )],
            content: vec![SvgText::Content(self.t.render_item(item))],
        })));
    }

    fn render_glyph(&mut self, pos: Scalar, glyph: &ir::GlyphItem) {
        let glyph_ref = self.t.glyph_pack.build_glyph(glyph);
        self.render_glyph_ref_inner(pos, &glyph_ref)
    }

    fn render_link(&mut self, link: &ir::LinkItem) {
        let href_handler = if link.href.starts_with("@typst:") {
            let href = link.href.trim_start_matches("@typst:");
            format!(r##"xlink:href="#" onclick="{href}; return false""##)
        } else {
            format!(
                r##"target="_blank" xlink:href="{}""##,
                link.href.replace('&', "&amp;")
            )
        };

        self.content.push(SvgText::Plain(format!(
            r#"<a {}><rect class="pseudo-link" width="{}" height="{}"></rect></a>"#,
            href_handler, link.size.x.0, link.size.y.0,
        )))
    }

    fn render_path(&mut self, path: &ir::PathItem) {
        self.content.push(SvgText::Plain(render_path(path)))
    }

    fn render_image(&mut self, image_item: &ir::ImageItem) {
        self.content
            .push(SvgText::Plain(render_image(&image_item.image, image_item.size)))
    }
    fn render_semantic_text(&mut self, text: &ir::TextItem, width: Scalar) {
        if !(Feat::SHOULD_RENDER_TEXT_ELEMENT && self.t.should_render_text_element) {
            return;
        }

        self.render_text_semantics_inner(&text.shape, &text.content.content, width)
    }
}

/// Render a [`PathItem`] into svg text.
#[comemo::memoize]
fn render_path(path: &PathItem) -> String {
    let mut p = vec!["<path ".to_owned()];
    p.push(format!(r#"d="{}" "#, path.d));
    let mut fill_color = "none";
    for style in &path.styles {
        match style {
            PathStyle::Fill(color) => {
                fill_color = color;
            }
            PathStyle::Stroke(color) => {
                p.push(format!(r#"stroke="{}" "#, color));
            }
            PathStyle::StrokeWidth(width) => {
                p.push(format!(r#"stroke-width="{}" "#, width.0));
            }
            PathStyle::StrokeLineCap(cap) => {
                p.push(format!(r#"stroke-linecap="{}" "#, cap));
            }
            PathStyle::StrokeLineJoin(join) => {
                p.push(format!(r#"stroke-linejoin="{}" "#, join));
            }
            PathStyle::StrokeMitterLimit(limit) => {
                p.push(format!(r#"stroke-miterlimit="{}" "#, limit.0));
            }
            PathStyle::StrokeDashArray(array) => {
                p.push(r#"stroke-dasharray="#.to_owned());
                for (i, v) in array.iter().enumerate() {
                    if i > 0 {
                        p.push(" ".to_owned());
                    }
                    p.push(format!("{}", v.0));
                }
                p.push(r#"" "#.to_owned());
            }
            PathStyle::StrokeDashOffset(offset) => {
                p.push(format!(r#"stroke-dashoffset="{}" "#, offset.0));
            }
        }
    }
    p.push(format!(r#"fill="{}" "#, fill_color));
    p.push("/>".to_owned());
    p.join("")
}

/// Render a raster or SVG image into svg text.
// todo: error handling
pub fn render_image(image: &Image, size: Size) -> String {
    let image_url = rasterize_embedded_image_url(image).unwrap();

    let w = size.x.0;
    let h = size.y.0;
    format!(
        r#"<image x="0" y="0" width="{}" height="{}" style="fill" xlink:href="{}" preserveAspectRatio="none" />"#,
        w, h, image_url
    )
}

fn rasterize_embedded_image_url(image: &Image) -> Option<String> {
    let url = format!("data:image/{};base64,", image.format);

    let mut data = base64::engine::general_purpose::STANDARD.encode(&image.data);
    data.insert_str(0, &url);
    Some(data)
}

/// Concatenate a list of [`SvgText`] into a single string.
pub fn generate_text(text_list: Vec<SvgText>) -> String {
    let mut string_io = String::new();
    string_io.reserve(text_list.iter().map(SvgText::estimated_len).sum());
    for s in text_list {
        s.write_string_io(&mut string_io);
    }
    string_io
}

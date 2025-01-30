use std::io::Read;

use base64::Engine;
use ecow::EcoString;
use ttf_parser::GlyphId;
use typst_library::foundations::{Bytes, Smart};
use typst_library::layout::{Abs, Point, Ratio, Size, Transform};
use typst_library::text::{Font, TextItem};
use typst_library::visualize::{
    ExchangeFormat, FillRule, Image, Paint, RasterImage, RelativeTo,
};
use typst_utils::hash128;

use crate::{SVGRenderer, State, SvgMatrix, SvgPathBuilder};

impl SVGRenderer {
    /// Render a text item. The text is rendered as a group of glyphs. We will
    /// try to render the text as SVG first, then bitmap, then outline. If none
    /// of them works, we will skip the text.
    pub(super) fn render_text(&mut self, state: State, text: &TextItem) {
        let scale: f64 = text.size.to_pt() / text.font.units_per_em();

        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-text");
        self.xml.write_attribute("transform", "scale(1, -1)");

        let mut x: f64 = 0.0;
        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let offset = x + glyph.x_offset.at(text.size).to_pt();

            self.render_svg_glyph(text, id, offset, scale)
                .or_else(|| self.render_bitmap_glyph(text, id, offset))
                .or_else(|| {
                    self.render_outline_glyph(
                        state
                            .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()))
                            .pre_translate(Point::new(Abs::pt(offset), Abs::zero())),
                        text,
                        id,
                        offset,
                        scale,
                    )
                });

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
        scale: f64,
    ) -> Option<()> {
        let data_url = convert_svg_glyph_to_base64_url(&text.font, id)?;
        let upem = text.font.units_per_em();
        let origin_ascender = text.font.metrics().ascender.at(Abs::pt(upem));

        let glyph_hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(glyph_hash, || RenderedGlyph::Image {
            url: data_url,
            width: upem,
            height: upem,
            ts: Transform::translate(Abs::zero(), -origin_ascender)
                .post_concat(Transform::scale(Ratio::new(scale), Ratio::new(-scale))),
        });

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml.write_attribute("x", &x_offset);
        self.xml.end_element();

        Some(())
    }

    /// Render a glyph defined by a bitmap.
    fn render_bitmap_glyph(
        &mut self,
        text: &TextItem,
        id: GlyphId,
        x_offset: f64,
    ) -> Option<()> {
        let (image, bitmap_x_offset, bitmap_y_offset) =
            convert_bitmap_glyph_to_image(&text.font, id)?;

        let glyph_hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(glyph_hash, || {
            let width = image.width();
            let height = image.height();
            let url = crate::image::convert_image_to_base64_url(&image);
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
        let scale_factor = target_height / image.height();
        self.xml.write_attribute("x", &(x_offset / scale_factor));
        self.xml.write_attribute_fmt(
            "transform",
            format_args!("scale({scale_factor} -{scale_factor})",),
        );
        self.xml.end_element();

        Some(())
    }

    /// Render a glyph defined by an outline.
    fn render_outline_glyph(
        &mut self,
        state: State,
        text: &TextItem,
        glyph_id: GlyphId,
        x_offset: f64,
        scale: f64,
    ) -> Option<()> {
        let scale = Ratio::new(scale);
        let path = convert_outline_glyph_to_path(&text.font, glyph_id, scale)?;
        let hash = hash128(&(&text.font, glyph_id, scale));
        let id = self.glyphs.insert_with(hash, || RenderedGlyph::Path(path));

        let glyph_size = text.font.ttf().glyph_bounding_box(glyph_id)?;
        let width = glyph_size.width() as f64 * scale.get();
        let height = glyph_size.height() as f64 * scale.get();

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml.write_attribute_fmt("x", format_args!("{x_offset}"));
        self.write_fill(
            &text.fill,
            FillRule::default(),
            Size::new(Abs::pt(width), Abs::pt(height)),
            self.text_paint_transform(state, &text.fill),
        );
        if let Some(stroke) = &text.stroke {
            self.write_stroke(
                stroke,
                Size::new(Abs::pt(width), Abs::pt(height)),
                self.text_paint_transform(state, &stroke.paint),
            );
        }
        self.xml.end_element();

        Some(())
    }

    fn text_paint_transform(&self, state: State, paint: &Paint) -> Transform {
        match paint {
            Paint::Solid(_) => Transform::identity(),
            Paint::Gradient(gradient) => match gradient.unwrap_relative(true) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => Transform::scale(
                    Ratio::new(state.size.x.to_pt()),
                    Ratio::new(state.size.y.to_pt()),
                )
                .post_concat(state.transform.invert().unwrap()),
            },
            Paint::Tiling(tiling) => match tiling.unwrap_relative(true) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => state.transform.invert().unwrap(),
            },
        }
    }

    /// Build the glyph definitions.
    pub(super) fn write_glyph_defs(&mut self) {
        if self.glyphs.is_empty() {
            return;
        }

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
}

/// Represents a glyph to be rendered.
pub enum RenderedGlyph {
    /// A path is a sequence of drawing commands.
    ///
    /// It is in the format of `M x y L x y C x1 y1 x2 y2 x y Z`.
    Path(EcoString),
    /// An image is a URL to an image file, plus the size and transform.
    ///
    /// The url is in the format of `data:image/{format};base64,`.
    Image { url: EcoString, width: f64, height: f64, ts: Transform },
}

/// Convert an outline glyph to an SVG path.
#[comemo::memoize]
fn convert_outline_glyph_to_path(
    font: &Font,
    id: GlyphId,
    scale: Ratio,
) -> Option<EcoString> {
    let mut builder = SvgPathBuilder::with_scale(scale);
    font.ttf().outline_glyph(id, &mut builder)?;
    Some(builder.0)
}

/// Convert a bitmap glyph to an encoded image URL.
#[comemo::memoize]
fn convert_bitmap_glyph_to_image(font: &Font, id: GlyphId) -> Option<(Image, f64, f64)> {
    let raster = font.ttf().glyph_raster_image(id, u16::MAX)?;
    if raster.format != ttf_parser::RasterImageFormat::PNG {
        return None;
    }
    let image = Image::new(
        RasterImage::new(Bytes::new(raster.data.to_vec()), ExchangeFormat::Png).ok()?,
        None,
        Smart::Auto,
    );
    Some((image, raster.x as f64, raster.y as f64))
}

/// Convert an SVG glyph to an encoded image URL.
#[comemo::memoize]
fn convert_svg_glyph_to_base64_url(font: &Font, id: GlyphId) -> Option<EcoString> {
    let mut data = font.ttf().glyph_svg_image(id)?.data;

    // Decompress SVGZ.
    let mut decoded = vec![];
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        decoder.read_to_end(&mut decoded).ok()?;
        data = &decoded;
    }

    let upem = font.units_per_em();
    let width = upem;
    let height = upem;
    let origin_ascender = font.metrics().ascender.at(Abs::pt(upem));

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
            format!(r#" viewBox="0 {} {width} {height}""#, -origin_ascender.to_pt())
                .as_str(),
        );
    }

    let mut url: EcoString = "data:image/svg+xml;base64,".into();
    let b64_encoded =
        base64::engine::general_purpose::STANDARD.encode(svg_str.as_bytes());
    url.push_str(&b64_encoded);

    Some(url)
}

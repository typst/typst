//! Utilities for color font handling

use std::io::Read;

use ttf_parser::GlyphId;
use typst_syntax::Span;
use usvg::{Tree, TreeParsing, TreeWriting};

use crate::{
    layout::{Abs, Axes, Em, Frame, FrameItem, Point},
    text::{Glyph, TextItem, TextItemView},
    visualize::{Color, Image, Paint, Rgb},
};

use super::Font;

pub fn is_color_glyph(font: &Font, g: &Glyph) -> bool {
    let ttf = font.ttf();
    let glyph_id = GlyphId(g.id);
    ttf.glyph_raster_image(glyph_id, 160).is_some()
        || ttf.glyph_svg_image(glyph_id).is_some()
        || ttf.is_color_glyph(glyph_id)
}

/// A SVG document with information about its dimensions
pub struct SizedSvg {
    /// The declared width of the SVG
    pub width: f32,
    /// The declared height of the SVG
    pub height: f32,
    /// The computed bounding box of the root element
    pub bbox: usvg::Rect,
    /// The SVG document
    pub tree: Tree,
}

/// Retrieve and measure the SVG document for a given glyph, if it exists.
///
/// This function decodes compressed SVG if needed, and computes dimensions
/// of the glyph.
pub fn get_svg_glyph(text: &TextItem, glyph: GlyphId) -> Option<SizedSvg> {
    let mut data = text.font.ttf().glyph_svg_image(glyph)?.data;

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
    let root = document.root_element();

    // Parse SVG.
    let opts = usvg::Options::default();
    let mut tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;
    tree.calculate_bounding_boxes();
    let view_box = tree.view_box.rect;

    // If there's no viewbox defined, use the em square for our scale
    // transformation ...
    let upem = text.font.units_per_em() as f32;
    let (mut width, mut height) = (upem, upem);

    // ... but if there's a viewbox or width, use that.
    if root.has_attribute("viewBox") || root.has_attribute("width") {
        width = view_box.width();
    }

    // Same as for width.
    if root.has_attribute("viewBox") || root.has_attribute("height") {
        height = view_box.height();
    }

    // Compute the space we need to draw our glyph.
    // See https://github.com/RazrFalcon/resvg/issues/602 for why
    // using the svg size is problematic here.
    let mut bbox = usvg::BBox::default();
    if let Some(tree_bbox) = tree.root.bounding_box {
        bbox = bbox.expand(tree_bbox);
    }

    Some(SizedSvg { width, height, bbox: bbox.to_rect()?, tree })
}

/// Returns a frame with the glyph drawn inside
///
/// The glyphs are sized in font units, [`text.item.size`] is
/// not taken into account.
pub fn frame_for_glyph(text: &TextItemView, glyph: &Glyph) -> Frame {
    let ttf = text.item.font.ttf();
    let upem = Abs::pt(ttf.units_per_em() as f64);

    let mut frame = Frame::new(Axes::new(upem, upem), typst::layout::FrameKind::Soft);

    let ppem = text.item.size.to_pt() * 2.0;
    let glyph_id = GlyphId(glyph.id);
    if let Some(raster_image) = ttf.glyph_raster_image(glyph_id, ppem as u16) {
        let image = Image::new(
            raster_image.data.into(),
            typst::visualize::ImageFormat::Raster(typst::visualize::RasterFormat::Png),
            None,
        )
        .unwrap();
        let position = Point::zero();
        let aspect_ratio = image.width() / image.height();
        let size = Axes::new(upem, upem * aspect_ratio);
        frame.push(position, FrameItem::Image(image, size, Span::detached()));
    } else if ttf.glyph_svg_image(glyph_id).is_some() {
        let Some(SizedSvg { tree, bbox, .. }) =
            typst::text::color_font::get_svg_glyph(text.item, glyph_id)
        else {
            // Return an empty frame if we were not able to
            // parse and measure the SVG
            return frame;
        };

        let mut data = tree.to_string(&usvg::XmlOptions::default());

        let width = bbox.width() as f64;
        let height = bbox.height() as f64;
        let left = bbox.left() as f64;
        let top = bbox.top() as f64;
        let bottom = bbox.bottom() as f64;

        // The SVG coordinates and the font coordinates are not the same:
        // the Y axis is mirrored. But the origin of the axes are the same
        // (which means that the horizontal axis in the SVG document
        // corresponds to the baseline). See the reference for more details:
        // https://learn.microsoft.com/en-us/typography/opentype/spec/svg#coordinate-systems-and-glyph-metrics
        // If we used the SVG document as it is, svg2pdf would produce a
        // cropped glyph (only what is under the baseline would be visible).
        // So we need to embed the original SVG in another one that has the
        // exact dimensions of the glyph, with a transform to make it fit.
        // We also need to remove the viewBox, height and width attributes
        // from the inner SVG, otherwise usvg takes into account these
        // values to clip the embedded SVG.
        make_svg_unsized(&mut data);
        let wrapper_svg = format!(
            r#"
            <svg
                width="{width}"
                height="{height}"
                viewBox="0 0 {width} {height}"
                xmlns="http://www.w3.org/2000/svg">
                <g transform="matrix(1 0 0 1 {tx} {ty})">
                {inner}
                </g>
            </svg>
        "#,
            inner = data,
            tx = -left,
            ty = -top,
        );

        let image = Image::new(
            wrapper_svg.as_bytes().into(),
            typst::visualize::ImageFormat::Vector(typst::visualize::VectorFormat::Svg),
            None,
        )
        .unwrap();
        let position = Point::new(Abs::pt(left), Abs::pt(bottom));
        let size = Axes::new(Abs::pt(width), Abs::pt(height));
        frame.push(position, FrameItem::Image(image, size, Span::detached()));
    } else if ttf.is_color_glyph(glyph_id) {
        let mut painter = ColrPainter {
            text: text.item,
            frame: &mut frame,
            foreground: Color::BLACK,
            current_glyph: glyph_id,
        };
        ttf.paint_color_glyph(glyph_id, 0, &mut painter);
    }

    frame
}

/// Remove all size specifications (viewBox, width and height attributes) from a
/// SVG document
fn make_svg_unsized(svg: &mut String) {
    let mut viewbox_range = None;
    let mut width_range = None;
    let mut height_range = None;

    let mut s = unscanny::Scanner::new(svg);

    s.eat_until("<svg");
    s.eat_if("<svg");
    while !s.eat_if('>') {
        s.eat_whitespace();
        let start = s.cursor();
        let attr_name = s.eat_until('=').trim();
        s.eat(); // eat the equal
        s.eat(); // eat the quote
        let mut escaped = false;
        while escaped || !s.eat_if('"') {
            escaped = s.eat() == Some('\\');
        }
        match attr_name {
            "viewBox" => {
                viewbox_range = Some(start..s.cursor());
            }
            "width" => {
                width_range = Some(start..s.cursor());
            }
            "height" => {
                height_range = Some(start..s.cursor());
            }
            _ => {}
        }
    }

    /// Because we will remove some attributes, other ranges may need to be shifted
    /// This function returns a mutable reference to a range (a) if it should be shifted after
    /// another range (b) was deleted
    fn should_shift<'a>(
        a: &'a mut Option<std::ops::Range<usize>>,
        b: &std::ops::Range<usize>,
    ) -> Option<&'a mut std::ops::Range<usize>> {
        // Is a after b?
        let is_after = a.as_ref().map(|r| r.start > b.end).unwrap_or(false);
        if is_after {
            a.as_mut()
        } else {
            None
        }
    }

    // remove the viewBox attribute
    if let Some(range) = viewbox_range {
        svg.replace_range(range.clone(), "");

        let shift = range.len();
        if let Some(ref mut width_range) = should_shift(&mut width_range, &range) {
            width_range.start -= shift;
            width_range.end -= shift;
        }

        if let Some(ref mut height_range) = should_shift(&mut height_range, &range) {
            height_range.start -= shift;
            height_range.end -= shift;
        }
    }

    // remove the width attribute
    if let Some(range) = width_range {
        svg.replace_range(range.clone(), "");

        let shift = range.len();
        if let Some(ref mut height_range) = should_shift(&mut height_range, &range) {
            height_range.start -= shift;
            height_range.end -= shift;
        }
    }

    // remove the height attribute
    if let Some(range) = height_range {
        svg.replace_range(range, "");
    }
}
struct ColrPainter<'f, 't> {
    frame: &'f mut Frame,
    /// The original text item
    text: &'t TextItem,
    current_glyph: GlyphId,
    foreground: Color,
}

impl<'f, 't> ColrPainter<'f, 't> {
    fn paint(&mut self, color: Color) {
        self.frame.push(
            // With images, the position corresponds to the top-left corner,
            // but in the case of text it matches the baseline-left point.
            // Here, we move the glyph one unit down to compensate for that.
            Point::new(Abs::zero(), Abs::pt(self.text.font.units_per_em() as f64)),
            FrameItem::Text(TextItem {
                font: self.text.font.clone(),
                size: Abs::pt(self.text.font.units_per_em() as f64),
                fill: Paint::Solid(color),
                stroke: None,
                lang: self.text.lang,
                text: self.text.text.clone(),
                glyphs: vec![Glyph {
                    id: self.current_glyph.0,
                    x_advance: Em::zero(), // Advance is not relevant here as we will draw glyph on top of each other anyway
                    x_offset: Em::zero(),  // Same
                    range: 0..self.text.text.len() as u16,
                    span: (Span::detached(), 0),
                }],
            }),
        )
    }
}

impl<'f, 't> ttf_parser::colr::Painter for ColrPainter<'f, 't> {
    fn outline(&mut self, glyph_id: GlyphId) {
        self.current_glyph = glyph_id;
    }

    fn paint_foreground(&mut self) {
        self.paint(self.foreground)
    }

    fn paint_color(&mut self, color: ttf_parser::RgbaColor) {
        let color = Color::Rgb(Rgb::new(
            color.red as f32 / 255.0,
            color.green as f32 / 255.0,
            color.blue as f32 / 255.0,
            color.alpha as f32 / 255.0,
        ));
        self.paint(color);
    }
}

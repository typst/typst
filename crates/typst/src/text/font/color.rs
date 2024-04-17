//! Utilities for color font handling

use std::io::Read;

use ecow::EcoString;
use ttf_parser::GlyphId;
use usvg::{TreeParsing, TreeWriting};

use crate::layout::{Abs, Axes, Em, Frame, FrameItem, Point, Size};
use crate::syntax::Span;
use crate::text::{Font, Glyph, Lang, TextItem};
use crate::visualize::{Color, Image, Paint, Rgb};

/// Tells if a glyph is a color glyph or not in a given font.
pub fn is_color_glyph(font: &Font, g: &Glyph) -> bool {
    let ttf = font.ttf();
    let glyph_id = GlyphId(g.id);
    ttf.glyph_raster_image(glyph_id, 160).is_some()
        || ttf.glyph_svg_image(glyph_id).is_some()
        || ttf.is_color_glyph(glyph_id)
}

/// Returns a frame with the glyph drawn inside.
///
/// The glyphs are sized in font units, [`text.item.size`] is not taken into
/// account.
#[comemo::memoize]
pub fn frame_for_glyph(font: &Font, glyph_id: u16) -> Frame {
    let ttf = font.ttf();
    let upem = Abs::pt(ttf.units_per_em() as f64);
    let glyph_id = GlyphId(glyph_id);

    let mut frame = Frame::soft(Size::splat(upem));

    if let Some(raster_image) = ttf.glyph_raster_image(glyph_id, u16::MAX) {
        draw_raster_glyph(&mut frame, font, upem, raster_image);
    } else if ttf.glyph_svg_image(glyph_id).is_some() {
        draw_svg_glyph(&mut frame, upem, font, glyph_id);
    } else if ttf.is_color_glyph(glyph_id) {
        draw_colr_glyph(&mut frame, font, glyph_id);
    }

    frame
}

/// Draws a raster glyph in a frame.
fn draw_raster_glyph(
    frame: &mut Frame,
    font: &Font,
    upem: Abs,
    raster_image: ttf_parser::RasterGlyphImage,
) {
    let image = Image::new(
        raster_image.data.into(),
        typst::visualize::ImageFormat::Raster(typst::visualize::RasterFormat::Png),
        None,
    )
    .unwrap();

    // Apple Color emoji doesn't provide offset information (or at least
    // not in a way ttf-parser understands), so we artificially shift their
    // baseline to make it look good.
    let y_offset = if font.info().family.to_lowercase() == "apple color emoji" {
        20.0
    } else {
        -(raster_image.y as f64)
    };

    let position = Point::new(
        upem * raster_image.x as f64 / raster_image.pixels_per_em as f64,
        upem * y_offset / raster_image.pixels_per_em as f64,
    );
    let aspect_ratio = image.width() / image.height();
    let size = Axes::new(upem, upem * aspect_ratio);
    frame.push(position, FrameItem::Image(image, size, Span::detached()));
}

/// Draws a COLR glyph in a frame.
fn draw_colr_glyph(frame: &mut Frame, font: &Font, glyph_id: GlyphId) {
    let mut painter = ColrPainter { font, current_glyph: glyph_id, frame };
    font.ttf().paint_color_glyph(glyph_id, 0, &mut painter);
}

/// Draws COLR glyphs in a frame.
struct ColrPainter<'f, 't> {
    /// The frame in which to draw.
    frame: &'f mut Frame,
    /// The font of the text.
    font: &'t Font,
    /// The glyph that will be drawn the next time `ColrPainter::paint` is called.
    current_glyph: GlyphId,
}

impl<'f, 't> ColrPainter<'f, 't> {
    fn paint(&mut self, fill: Paint) {
        self.frame.push(
            // With images, the position corresponds to the top-left corner, but
            // in the case of text it matches the baseline-left point. Here, we
            // move the glyph one unit down to compensate for that.
            Point::new(Abs::zero(), Abs::pt(self.font.units_per_em())),
            FrameItem::Text(TextItem {
                font: self.font.clone(),
                size: Abs::pt(self.font.units_per_em()),
                fill,
                stroke: None,
                lang: Lang::ENGLISH,
                text: EcoString::new(),
                glyphs: vec![Glyph {
                    id: self.current_glyph.0,
                    // Advance is not relevant here as we will draw glyph on top
                    // of each other anyway
                    x_advance: Em::zero(),
                    x_offset: Em::zero(),
                    range: 0..0,
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
        // Default to black if no color was specified
        self.paint(Paint::Solid(Color::BLACK))
    }

    fn paint_color(&mut self, color: ttf_parser::RgbaColor) {
        let color = Color::Rgb(Rgb::new(
            color.red as f32 / 255.0,
            color.green as f32 / 255.0,
            color.blue as f32 / 255.0,
            color.alpha as f32 / 255.0,
        ));
        self.paint(Paint::Solid(color));
    }
}

/// Draws an SVG glyph in a frame.
fn draw_svg_glyph(
    frame: &mut Frame,
    upem: Abs,
    font: &Font,
    glyph_id: GlyphId,
) -> Option<()> {
    let mut data = font.ttf().glyph_svg_image(glyph_id)?.data;

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
    let mut tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;

    // Compute the space we need to draw our glyph.
    // See https://github.com/RazrFalcon/resvg/issues/602 for why
    // using the svg size is problematic here.
    tree.calculate_bounding_boxes();
    let mut bbox = usvg::BBox::default();
    if let Some(tree_bbox) = tree.root.bounding_box {
        bbox = bbox.expand(tree_bbox);
    }
    let bbox = bbox.to_rect()?;

    let mut data = tree.to_string(&usvg::XmlOptions::default());

    let width = bbox.width() as f64;
    let height = bbox.height() as f64;
    let left = bbox.left() as f64;
    let top = bbox.top() as f64;

    // The SVG coordinates and the font coordinates are not the same: the Y axis
    // is mirrored. But the origin of the axes are the same (which means that
    // the horizontal axis in the SVG document corresponds to the baseline). See
    // the reference for more details:
    // https://learn.microsoft.com/en-us/typography/opentype/spec/svg#coordinate-systems-and-glyph-metrics
    //
    // If we used the SVG document as it is, svg2pdf would produce a cropped
    // glyph (only what is under the baseline would be visible). So we need to
    // embed the original SVG in another one that has the exact dimensions of
    // the glyph, with a transform to make it fit. We also need to remove the
    // viewBox, height and width attributes from the inner SVG, otherwise usvg
    // takes into account these values to clip the embedded SVG.
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
        wrapper_svg.into_bytes().into(),
        typst::visualize::ImageFormat::Vector(typst::visualize::VectorFormat::Svg),
        None,
    )
    .unwrap();
    let position = Point::new(Abs::pt(left), Abs::pt(top) + upem);
    let size = Axes::new(Abs::pt(width), Abs::pt(height));
    frame.push(position, FrameItem::Image(image, size, Span::detached()));

    Some(())
}

/// Remove all size specifications (viewBox, width and height attributes) from a
/// SVG document.
fn make_svg_unsized(svg: &mut String) {
    let mut viewbox_range = None;
    let mut width_range = None;
    let mut height_range = None;

    let mut s = unscanny::Scanner::new(svg);

    s.eat_until("<svg");
    s.eat_if("<svg");
    while !s.eat_if('>') && !s.done() {
        s.eat_whitespace();
        let start = s.cursor();
        let attr_name = s.eat_until('=').trim();
        // Eat the equal sign and the quote.
        s.eat();
        s.eat();
        let mut escaped = false;
        while (escaped || !s.eat_if('"')) && !s.done() {
            escaped = s.eat() == Some('\\');
        }
        match attr_name {
            "viewBox" => viewbox_range = Some(start..s.cursor()),
            "width" => width_range = Some(start..s.cursor()),
            "height" => height_range = Some(start..s.cursor()),
            _ => {}
        }
    }

    // Remove the `viewBox` attribute.
    if let Some(range) = viewbox_range {
        svg.replace_range(range.clone(), &" ".repeat(range.len()));
    }

    // Remove the `width` attribute.
    if let Some(range) = width_range {
        svg.replace_range(range.clone(), &" ".repeat(range.len()));
    }

    // Remove the `height` attribute.
    if let Some(range) = height_range {
        svg.replace_range(range, "");
    }
}

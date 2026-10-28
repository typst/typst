//! Utilities for color font handling

use std::io::Read;

use read_fonts::TableProvider;
use read_fonts::model::glyph::color::{Brush, Color, ColorStop, Extend, PaintSink};
use read_fonts::model::glyph::outline::{
    ControlBoundsSink, DrawMemory, DrawOptions, PathSink, SvgSink, TransformSink,
};
use read_fonts::types::{BoundingBox, GlyphId, Matrix};
use typst_syntax::Span;
use xmlwriter::XmlWriter;

use crate::foundations::Bytes;
use crate::layout::{Abs, Frame, FrameItem, Point, Rect, Size};
use crate::text::FontInstance;
use crate::visualize::{
    ExchangeFormat, FixedStroke, Geometry, Image, RasterImage, Shape, SvgImage,
};

/// Whether this glyph should be rendered via simple outlining instead of via
/// `glyph_frame`.
pub fn should_outline(font: &FontInstance, glyph_id: GlyphId) -> bool {
    let instance = font.instance();
    !instance.outline_glyphs().is_empty()
        && !instance.bitmap_strikes().select_bitmap(glyph_id, None).is_some_and(
            |bitmap| {
                matches!(
                    bitmap.pixels(),
                    read_fonts::model::glyph::bitmap::Pixels::Png(_)
                )
            },
        )
        && instance.color_glyphs().get(glyph_id).is_none()
        && !instance
            .tables()
            .svg()
            .is_ok_and(|svg| svg.glyph_data(glyph_id).is_some())
}

/// A frame that can draw a glyph.
#[derive(Clone)]
pub struct GlyphFrame {
    pub upem: Abs,
    pub item: GlyphFrameItem,
}

impl GlyphFrame {
    /// The font unit square.
    pub fn size(&self) -> Size {
        Size::splat(self.upem)
    }
}

impl From<GlyphFrame> for Frame {
    fn from(g: GlyphFrame) -> Self {
        let mut frame = Frame::soft(Size::splat(g.upem));
        match g.item {
            GlyphFrameItem::Tofu(pos, shape) => {
                frame.push(pos, FrameItem::Shape(shape, Span::detached()));
            }
            GlyphFrameItem::Image(pos, image, size) => {
                frame.push(pos, FrameItem::Image(image, size, Span::detached()));
            }
        }
        frame
    }
}

/// The glyph item that is drawn.
#[derive(Clone)]
pub enum GlyphFrameItem {
    /// A fallback rectangle.
    Tofu(Point, Shape),
    /// An image glyph.
    Image(Point, Image, Size),
}

impl GlyphFrameItem {
    /// The position of the glyph item inside the parent frame.
    pub fn pos(&self) -> Point {
        match *self {
            GlyphFrameItem::Tofu(pos, _) => pos,
            GlyphFrameItem::Image(pos, _, _) => pos,
        }
    }
}

/// Returns a frame representing a glyph and whether it is a fallback tofu
/// frame.
///
/// Should only be called on glyphs for which [`should_outline`] returns false.
///
/// The glyphs are sized in font units, [`text.item.size`] is not taken into
/// account.
///
/// [`text.item.size`]: crate::text::TextItem::size
#[comemo::memoize]
pub fn glyph_frame(font: &FontInstance, glyph_id: u32) -> Option<GlyphFrame> {
    let upem = Abs::pt(font.units_per_em());
    let glyph_id = GlyphId::new(glyph_id);

    if let Some(frame) = draw_glyph(font, upem, glyph_id) {
        return Some(frame);
    }

    // Generate a fallback tofu if the glyph couldn't be drawn, unless it is
    // the space glyph. Then, an empty frame does the job. (This happens for
    // some rare CBDT fonts, which don't define a bitmap for the space, but
    // also don't have a glyf or CFF table.)
    let not_space = font.instance().charmap().map_unicode(' ') != Some(glyph_id);
    not_space.then(|| draw_fallback_tofu(font, upem, glyph_id))
}

/// Tries to draw a glyph.
fn draw_glyph(font: &FontInstance, upem: Abs, glyph_id: GlyphId) -> Option<GlyphFrame> {
    let instance = font.instance();
    let kind = if let Some(raster_image) = instance
        .bitmap_strikes()
        .select_bitmap(glyph_id, None)
        .filter(|bitmap| {
            matches!(bitmap.pixels(), read_fonts::model::glyph::bitmap::Pixels::Png(_))
        }) {
        draw_raster_glyph(font, upem, raster_image)
    } else if instance.color_glyphs().get(glyph_id).is_some() {
        draw_colr_glyph(font, glyph_id)
    } else if instance
        .tables()
        .svg()
        .is_ok_and(|svg| svg.glyph_data(glyph_id).is_some())
    {
        draw_svg_glyph(font, glyph_id)
    } else {
        None
    };

    kind.map(|kind| GlyphFrame { upem, item: kind })
}

/// Draws a fallback tofu box with the advance width of the glyph.
fn draw_fallback_tofu(font: &FontInstance, upem: Abs, glyph_id: GlyphId) -> GlyphFrame {
    let advance =
        Abs::pt(font.instance().glyph_metrics().h_advance_exact(glyph_id).to_f64());
    let inset = 0.15 * advance;
    let height = 0.7 * upem;
    let pos = Point::new(inset, upem - height);
    let size = Size::new(advance - inset * 2.0, height);
    let thickness = upem / 20.0;
    let stroke = FixedStroke { thickness, ..Default::default() };
    let shape = Geometry::Rect(size).stroked(stroke);
    GlyphFrame { upem, item: GlyphFrameItem::Tofu(pos, shape) }
}

/// Draws a raster glyph in a frame.
///
/// Supports only PNG images.
fn draw_raster_glyph(
    font: &FontInstance,
    upem: Abs,
    raster: read_fonts::model::glyph::bitmap::BitmapGlyph,
) -> Option<GlyphFrameItem> {
    let read_fonts::model::glyph::bitmap::Pixels::Png(data) = raster.pixels() else {
        unreachable!()
    };
    let data = Bytes::new(data.to_vec());
    let image = Image::plain(RasterImage::plain(data, ExchangeFormat::Png).ok()?);

    let scale = upem / raster.ppem() as f64;
    let image_width = scale * image.width();
    let image_height = scale * image.height();

    let x_offset = scale * raster.inner_x_bearing() as f64;
    let y = match raster.origin() {
        read_fonts::model::glyph::bitmap::Origin::TopLeft => {
            raster.inner_y_bearing() as f64 - raster.height() as f64
        }
        read_fonts::model::glyph::bitmap::Origin::BottomLeft => {
            raster.inner_y_bearing() as f64
        }
    };
    let mut y_offset = scale * y;
    // Apple Color emoji doesn't provide offset information (or at least
    // not in a way read-fonts understands), so we artificially shift their
    // baseline to make it look good.
    if font.info().family.to_lowercase() == "apple color emoji" {
        // This factor is taken from krilla.
        y_offset -= 0.125 * upem;
    }

    let position = Point::new(-x_offset, -(image_height + y_offset));
    let size = Size::new(image_width, image_height);
    Some(GlyphFrameItem::Image(position, image, size))
}

/// Draws a glyph from the COLR table into the frame.
fn draw_colr_glyph(font: &FontInstance, glyph_id: GlyphId) -> Option<GlyphFrameItem> {
    let svg_string = colr_glyph_to_svg(font, glyph_id)?;

    let bounds = font.instance().metrics().bounds;
    let width = (bounds.x_max - bounds.x_min).to_f64();
    let height = (bounds.y_max - bounds.y_min).to_f64();
    let x_min = bounds.x_min.to_f64();
    let y_max = bounds.y_max.to_f64();

    let data = Bytes::from_string(svg_string);
    let image = Image::plain(SvgImage::new(data).ok()?);

    let position = Point::new(Abs::pt(x_min), Abs::pt(-y_max));
    let size = Size::new(Abs::pt(width), Abs::pt(height));
    Some(GlyphFrameItem::Image(position, image, size))
}

/// Convert a COLR glyph into an SVG file.
fn colr_glyph_to_svg(font: &FontInstance, glyph_id: GlyphId) -> Option<String> {
    let mut svg = XmlWriter::new(xmlwriter::Options::default());

    let instance = font.instance();

    let bounds = instance.metrics().bounds;
    let width = (bounds.x_max - bounds.x_min).to_f64();
    let height = (bounds.y_max - bounds.y_min).to_f64();
    let x_min = bounds.x_min.to_f64();
    let y_max = bounds.y_max.to_f64();
    let tx = -x_min;
    let ty = -y_max;

    svg.start_element("svg");
    svg.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    svg.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    svg.write_attribute("width", &width);
    svg.write_attribute("height", &height);
    svg.write_attribute_fmt("viewBox", format_args!("0 0 {width} {height}"));

    let gradient_index = 1;
    let clip_path_index = 1;

    svg.start_element("g");
    svg.write_attribute_fmt(
        "transform",
        format_args!("matrix(1 0 0 -1 0 0) matrix(1 0 0 1 {tx} {ty})"),
    );

    let mut glyph_painter = GlyphPainter {
        font: instance,
        svg: &mut svg,
        path: SvgSink::new(),
        gradient_index,
        clip_path_index,
        // We always use the first palette. `ColorPalettes` handles
        // per-palette record offsets internally.
        palette: instance.palettes().get(0).map_or(&[][..], |palette| palette.colors()),
        foreground_color: Color { red: 0, green: 0, blue: 0, alpha: 255 },
        transform: Matrix::IDENTITY,
        outline_transform: Matrix::IDENTITY,
        transforms_stack: vec![Matrix::IDENTITY],
        clip_stack: vec![],
    };

    instance
        .color_glyphs()
        .get(glyph_id)?
        .paint(&mut glyph_painter)
        .ok()?;
    svg.end_element();

    Some(svg.end_document())
}

/// Draws an SVG glyph in a frame.
fn draw_svg_glyph(font: &FontInstance, glyph_id: GlyphId) -> Option<GlyphFrameItem> {
    // TODO: Our current conversion of the SVG table works for Twitter Color Emoji,
    // but might not work for others. See also: https://github.com/RazrFalcon/resvg/pull/776
    let mut data = font
        .instance()
        .tables()
        .svg()
        .ok()
        .and_then(|svg| svg.glyph_data(glyph_id))?;

    // Decompress SVGZ.
    let mut decoded = vec![];
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        decoder.read_to_end(&mut decoded).ok()?;
        data = &decoded;
    }

    // Parse and simplify the SVG.
    let xml = std::str::from_utf8(data).ok()?;
    let document = roxmltree::Document::parse(xml).ok()?;
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;
    let mut data = tree.to_string(&usvg::WriteOptions {
        indent: usvg::Indent::None,
        attributes_indent: usvg::Indent::None,
        ..Default::default()
    });

    // The SVG coordinates and the font coordinates are not the same: the Y axis
    // is mirrored. But the origin of the axes are the same (which means that
    // the horizontal axis in the SVG document corresponds to the baseline). See
    // the reference for more details:
    // https://learn.microsoft.com/en-us/typography/opentype/spec/svg#coordinate-systems-and-glyph-metrics
    //
    // Using this SVG directly can result in a cropped glyph. In order to avoid
    // clipping issues, we apply a translate transform so the top-left corner of
    // the bounding box is moved into the origin (0, 0) to make it fully fit
    // into the view port defined by `viewBox="0 0 width height"`, like a
    // conventional SVG.
    let bbox = tree.root().bounding_box();
    let view_box = Rect::new(
        Point::new(Abs::pt(bbox.left() as f64), Abs::pt(bbox.top() as f64)),
        Point::new(Abs::pt(bbox.right() as f64), Abs::pt(bbox.bottom() as f64)),
    );
    fixup_svg(&mut data, view_box);

    let data = Bytes::from_string(data);
    let image = Image::plain(SvgImage::new(data).ok()?);

    let position = Point::new(view_box.min.x, view_box.min.y);
    let size = view_box.size();
    Some(GlyphFrameItem::Image(position, image, size))
}

/// Replace or insert the size attributes (viewBox, width and height), and
/// insert a group with a transform that translates the `viewBox` so that the
/// top-left point is at the origin (0, 0).
fn fixup_svg(svg: &mut String, view_box: Rect) {
    let mut viewbox_range = None;
    let mut width_range = None;
    let mut height_range = None;

    let mut s = unscanny::Scanner::new(svg);
    s.eat_until("<svg");
    s.expect("<svg");

    let svg_attr_start = s.cursor();

    while !s.eat_if('>') && !s.done() {
        s.eat_whitespace();
        let start = s.cursor();

        let attr_name = s.eat_until('=').trim();
        // Eat the equal sign and the quote.
        s.expect('=');
        s.eat_until('"');
        s.expect('"');

        while !s.eat_if('"') && !s.done() {
            s.eat();
        }

        match attr_name {
            "viewBox" => viewbox_range = Some(start..s.cursor()),
            "width" => width_range = Some(start..s.cursor()),
            "height" => height_range = Some(start..s.cursor()),
            _ => {}
        }
    }

    let svg_body_start = s.cursor();
    let Some(svg_body_end) = svg.rfind("</svg>") else {
        return;
    };

    svg.insert_str(svg_body_end, "</g>");
    svg.insert_str(
        svg_body_start,
        &format!(
            r#"<g transform="translate({} {})">"#,
            -view_box.min.x.to_pt(),
            -view_box.min.y.to_pt()
        ),
    );

    let size = view_box.size();
    let mut edits = [
        (
            viewbox_range,
            format!("viewBox=\"0 0 {} {}\"", size.x.to_pt(), size.y.to_pt(),),
        ),
        (width_range, format!("width=\"{}\"", size.x.to_pt())),
        (height_range, format!("height=\"{}\"", size.y.to_pt())),
    ];

    // Sort edits by ranges; missing ranges will be moved to the start.
    edits.sort_by_key(|(range, _)| range.clone().map(|r| r.start));

    // Replace or insert the attribute. Iterate in reverse, so the modifying the
    // string doesn't affect the ranges of the edits that are applied later on.
    for (range, str) in edits.into_iter().rev() {
        if let Some(range) = range {
            svg.replace_range(range, &str);
        } else {
            svg.insert_str(svg_attr_start, &str);
            svg.insert(svg_attr_start, ' ');
        }
    }
}

/// Writes an outline as SVG path data and collects its control bounds in the
/// root coordinate space at the same time.
struct ColrSink<'a> {
    path: &'a mut SvgSink,
    bounds: TransformSink<ControlBoundsSink>,
}

impl<'a> ColrSink<'a> {
    /// Starts a new path that is drawn under `transform`.
    fn new(path: &'a mut SvgSink, transform: Matrix<f32>) -> Self {
        path.clear();
        Self {
            path,
            bounds: TransformSink::new(ControlBoundsSink::new(), transform),
        }
    }

    /// The control bounds of the path in the root coordinate space, or `None`
    /// if the path is empty.
    fn bounds(self) -> Option<BoundingBox<f32>> {
        self.bounds.into_inner().bounding_box()
    }
}

impl PathSink for ColrSink<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(x, y);
        self.bounds.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(x, y);
        self.bounds.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(cx0, cy0, x, y);
        self.bounds.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.curve_to(cx0, cy0, cx1, cy1, x, y);
        self.bounds.curve_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.path.close();
        self.bounds.close();
    }
}

// NOTE: This is only a best-effort translation of COLR into SVG. It's not feature-complete
// and it's also not possible to make it feature-complete using just raw SVG features.
struct GlyphPainter<'a> {
    font: &'a read_fonts::model::Font,
    svg: &'a mut XmlWriter,
    path: SvgSink,
    gradient_index: usize,
    clip_path_index: usize,
    /// The colors of the first palette.
    palette: &'a [Color],
    foreground_color: Color,
    transform: Matrix<f32>,
    outline_transform: Matrix<f32>,
    transforms_stack: Vec<Matrix<f32>>,
    /// The bounding box of every active clip, in the root coordinate space.
    /// `None` means the clip is empty (or its bounds are unknown).
    clip_stack: Vec<Option<BoundingBox<f32>>>,
}

impl GlyphPainter<'_> {
    /// Outlines a glyph into `path_buf` at the current variation location
    /// (an empty path on failure), records the current transform as the
    /// outline transform and returns the outline's conservative bounding box
    /// in the glyph's local coordinate space.
    fn outline_glyph(&mut self, glyph_id: GlyphId) -> Option<BoundingBox<f32>> {
        let mut sink = ColrSink::new(&mut self.path, self.transform);
        let outlined = self.font.outline_glyphs().get(glyph_id).is_some_and(|outline| {
            outline
                .draw(&DrawOptions::new(), DrawMemory::Scratch, &mut sink)
                .is_ok()
        });
        if !outlined {
            // A partial outline may have been written before a draw error.
            self.path.clear();
            return None;
        }

        // We have to write outline using the current transform.
        self.outline_transform = self.transform;

        sink.bounds()
    }

    fn push_clip(&mut self, bounds: Option<BoundingBox<f32>>) {
        let clip_id = format!("cp{}", self.clip_path_index);
        self.clip_path_index += 1;

        self.svg.start_element("clipPath");
        self.svg.write_attribute("id", &clip_id);
        self.svg.start_element("path");
        write_transform_attribute(self.svg, "transform", self.outline_transform);
        self.svg.write_attribute("d", &*self.path);
        self.svg.end_element();
        self.svg.end_element();

        self.svg.start_element("g");
        self.svg
            .write_attribute_fmt("clip-path", format_args!("url(#{clip_id})"));

        self.clip_stack.push(bounds);
    }

    /// Paints `path_buf` (positioned by the outline transform) with the given brush.
    fn paint_brush(&mut self, brush: Brush<'_>) {
        match brush {
            Brush::Solid { palette_index, alpha } => {
                let color = self.palette_index_to_color(palette_index, alpha);
                self.paint_solid(color);
            }
            Brush::LinearGradient { p0, p1, color_stops, extend } => {
                self.paint_linear_gradient(p0, p1, color_stops, extend);
            }
            Brush::RadialGradient { c0, r0, c1, r1, color_stops, extend } => {
                self.paint_radial_gradient(c0, r0, c1, r1, color_stops, extend);
            }
            Brush::SweepGradient { c0, start_angle, end_angle, color_stops, extend } => {
                self.paint_sweep_gradient(
                    c0,
                    start_angle,
                    end_angle,
                    color_stops,
                    extend,
                );
            }
        }
    }

    fn palette_index_to_color(&self, palette_index: u16, alpha: f32) -> Color {
        let mut color = if palette_index == u16::MAX {
            self.foreground_color
        } else {
            self.palette
                .get(palette_index as usize)
                .copied()
                .unwrap_or(self.foreground_color)
        };

        // Multiply alpha
        color.alpha = ((color.alpha as f32) * alpha) as u8;

        color
    }

    fn paint_solid(&mut self, color: Color) {
        self.svg.start_element("path");
        write_color_attribute(self.svg, "fill", color);
        let opacity = f32::from(color.alpha) / 255.0;
        self.svg.write_attribute("fill-opacity", &opacity);
        write_transform_attribute(self.svg, "transform", self.outline_transform);
        self.svg.write_attribute("d", &*self.path);
        self.svg.end_element();
    }

    fn paint_linear_gradient(
        &mut self,
        p0: read_fonts::types::Point<f32>,
        p1: read_fonts::types::Point<f32>,
        color_stops: &[ColorStop],
        extend: Extend,
    ) {
        let gradient_id = format!("lg{}", self.gradient_index);
        self.gradient_index += 1;

        let gradient_transform = paint_transform(self.outline_transform, self.transform);

        self.svg.start_element("linearGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("x1", &p0.x);
        self.svg.write_attribute("y1", &p0.y);
        self.svg.write_attribute("x2", &p1.x);
        self.svg.write_attribute("y2", &p1.y);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        write_spread_method_attribute(self.svg, extend);
        write_transform_attribute(self.svg, "gradientTransform", gradient_transform);
        self.write_gradient_stops(color_stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{gradient_id})"));
        write_transform_attribute(self.svg, "transform", self.outline_transform);
        self.svg.write_attribute("d", &*self.path);
        self.svg.end_element();
    }

    fn paint_radial_gradient(
        &mut self,
        c0: read_fonts::types::Point<f32>,
        r0: f32,
        c1: read_fonts::types::Point<f32>,
        r1: f32,
        color_stops: &[ColorStop],
        extend: Extend,
    ) {
        let gradient_id = format!("rg{}", self.gradient_index);
        self.gradient_index += 1;

        let gradient_transform = paint_transform(self.outline_transform, self.transform);

        // TODO: Normalizing the stops into the 0..1 range moves the circles onto the
        // first and last stop, which can make `r0` (and in theory `r1`) negative.
        // SVG cannot express that, so the color line should be cut where the radius
        // reaches zero, with an interpolated stop inserted at the cut and the
        // remaining stops reparameterized into the 0..1 range.
        self.svg.start_element("radialGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("cx", &c1.x);
        self.svg.write_attribute("cy", &c1.y);
        self.svg.write_attribute("r", &r1);
        self.svg.write_attribute("fr", &r0);
        self.svg.write_attribute("fx", &c0.x);
        self.svg.write_attribute("fy", &c0.y);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        write_spread_method_attribute(self.svg, extend);
        write_transform_attribute(self.svg, "gradientTransform", gradient_transform);
        self.write_gradient_stops(color_stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{gradient_id})"));
        write_transform_attribute(self.svg, "transform", self.outline_transform);
        self.svg.write_attribute("d", &*self.path);
        self.svg.end_element();
    }

    fn paint_sweep_gradient(
        &mut self,
        _c0: read_fonts::types::Point<f32>,
        _start_angle: f32,
        _end_angle: f32,
        _color_stops: &[ColorStop],
        _extend: Extend,
    ) {
    }

    fn write_gradient_stops(&mut self, stops: &[ColorStop]) {
        for stop in stops {
            let color = self.palette_index_to_color(stop.palette_index, stop.alpha);
            self.svg.start_element("stop");
            self.svg.write_attribute("offset", &stop.offset);
            write_color_attribute(self.svg, "stop-color", color);
            let opacity = f32::from(color.alpha) / 255.0;
            self.svg.write_attribute("stop-opacity", &opacity);
            self.svg.end_element();
        }
    }
}

impl PaintSink for GlyphPainter<'_> {
    fn push_layer(
        &mut self,
        composite_mode: read_fonts::model::glyph::color::CompositeMode,
    ) {
        self.svg.start_element("g");

        use read_fonts::model::glyph::color::CompositeMode;
        // TODO: Need to figure out how to represent the other blend modes
        // in SVG.
        let mode = match composite_mode {
            CompositeMode::SrcOver => "normal",
            CompositeMode::Screen => "screen",
            CompositeMode::Overlay => "overlay",
            CompositeMode::Darken => "darken",
            CompositeMode::Lighten => "lighten",
            CompositeMode::ColorDodge => "color-dodge",
            CompositeMode::ColorBurn => "color-burn",
            CompositeMode::HardLight => "hard-light",
            CompositeMode::SoftLight => "soft-light",
            CompositeMode::Difference => "difference",
            CompositeMode::Exclusion => "exclusion",
            CompositeMode::Multiply => "multiply",
            CompositeMode::HslHue => "hue",
            CompositeMode::HslSaturation => "saturation",
            CompositeMode::HslColor => "color",
            CompositeMode::HslLuminosity => "luminosity",
            _ => "normal",
        };
        self.svg.write_attribute_fmt(
            "style",
            format_args!("mix-blend-mode: {mode}; isolation: isolate"),
        );
    }

    fn pop_layer(&mut self) {
        self.svg.end_element(); // g
    }

    fn push_transform(&mut self, transform: Matrix<f32>) {
        self.transforms_stack.push(self.transform);
        self.transform *= transform;
    }

    fn fill_glyph(
        &mut self,
        glyph_id: GlyphId,
        brush_transform: Option<Matrix<f32>>,
        brush: Brush<'_>,
    ) {
        // Fill the glyph outline directly instead of the default
        // clip-then-fill decomposition. This avoids a redundant clip path
        // per fill and matches the output of the old ttf-parser based painter.
        self.outline_glyph(glyph_id);

        if let Some(brush_transform) = brush_transform {
            self.push_transform(brush_transform);
            self.paint_brush(brush);
            self.pop_transform();
        } else {
            self.paint_brush(brush);
        }
    }

    fn pop_transform(&mut self) {
        if let Some(ts) = self.transforms_stack.pop() {
            self.transform = ts;
        }
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let bounds = self.outline_glyph(glyph_id);

        // Clip with the outline. This must always open a clip group - even
        // when outlining failed (an empty path clips everything away) - since
        // the corresponding `pop_clip` will unconditionally close it.
        self.push_clip(bounds);
    }

    fn push_clip_box(&mut self, clipbox: BoundingBox<f32>) {
        // The clip box is positioned by the current transform.
        self.outline_transform = self.transform;
        let mut sink = ColrSink::new(&mut self.path, self.transform);
        sink.move_to(clipbox.x_min, clipbox.y_min);
        sink.line_to(clipbox.x_max, clipbox.y_min);
        sink.line_to(clipbox.x_max, clipbox.y_max);
        sink.line_to(clipbox.x_min, clipbox.y_max);
        sink.close();

        let bounds = sink.bounds();
        self.push_clip(bounds);
    }

    fn pop_clip(&mut self) {
        self.svg.end_element();
        self.clip_stack.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        // A fill paints the intersection of all currently active clips.
        // Paint a rectangle covering that intersection and let the enclosing
        // clip groups shape it.

        let mut region: Option<BoundingBox<f32>> = None;
        for bounds in &self.clip_stack {
            // A clip with no (or unknown) bounds clips everything away.
            let Some(bounds) = bounds else { return };
            region = Some(match region {
                Some(region) => match intersect_bboxs(region, *bounds) {
                    Some(r) => r,
                    // An empty intersection - there is nothing to paint.
                    None => return,
                },
                None => *bounds,
            });
        }
        let Some(region) = region else { return };

        let mut sink = ColrSink::new(&mut self.path, Matrix::IDENTITY);
        sink.move_to(region.x_min, region.y_min);
        sink.line_to(region.x_max, region.y_min);
        sink.line_to(region.x_max, region.y_max);
        sink.line_to(region.x_min, region.y_max);
        sink.close();

        // The covering rectangle is in the root coordinate space.
        self.outline_transform = Matrix::IDENTITY;

        self.paint_brush(brush);
    }
}

fn write_transform_attribute(svg: &mut XmlWriter, name: &str, ts: Matrix<f32>) {
    if ts != Matrix::IDENTITY {
        svg.write_attribute_fmt(
            name,
            format_args!(
                "matrix({} {} {} {} {} {})",
                ts.xx, ts.yx, ts.xy, ts.yy, ts.dx, ts.dy
            ),
        );
    }
}

fn write_spread_method_attribute(svg: &mut XmlWriter, extend: Extend) {
    svg.write_attribute(
        "spreadMethod",
        match extend {
            Extend::Pad => &"pad",
            Extend::Repeat => &"repeat",
            Extend::Reflect => &"reflect",
            Extend::Unknown => return,
        },
    );
}

fn write_color_attribute(svg: &mut XmlWriter, name: &str, color: Color) {
    svg.write_attribute_fmt(
        name,
        format_args!("rgb({}, {}, {})", color.red, color.green, color.blue),
    );
}

fn paint_transform(
    outline_transform: Matrix<f32>,
    transform: Matrix<f32>,
) -> Matrix<f32> {
    let to_tsp = |m: Matrix<f32>| {
        usvg::tiny_skia_path::Transform::from_row(m.xx, m.yx, m.xy, m.yy, m.dx, m.dy)
    };
    let tsp = to_tsp(outline_transform)
        .invert()
        // In theory, we should error out. But the transform shouldn't ever be uninvertible, so let's ignore it.
        .unwrap_or_default()
        .pre_concat(to_tsp(transform));
    Matrix {
        xx: tsp.sx,
        yx: tsp.ky,
        xy: tsp.kx,
        yy: tsp.sy,
        dx: tsp.tx,
        dy: tsp.ty,
    }
}

/// Returns the intersection of two bboxs, or `None` when they do not overlap.
fn intersect_bboxs(a: BoundingBox<f32>, b: BoundingBox<f32>) -> Option<BoundingBox<f32>> {
    let r = BoundingBox {
        x_min: a.x_min.max(b.x_min),
        y_min: a.y_min.max(b.y_min),
        x_max: a.x_max.min(b.x_max),
        y_max: a.y_max.min(b.y_max),
    };
    (r.x_min <= r.x_max && r.y_min <= r.y_max).then_some(r)
}

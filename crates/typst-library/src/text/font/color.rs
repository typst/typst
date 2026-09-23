//! Utilities for color font handling

use std::io::Read;

use skrifa::color::{Brush, Color, ColorStop, Extend, Transform};
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use typst_syntax::Span;
use usvg::tiny_skia_path;
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
    let skrifa = font.skrifa();
    skrifa.outline_glyphs().format().is_some()
        && !skrifa
            .bitmap_strikes()
            .glyph_for_size(skrifa::instance::Size::unscaled(), glyph_id)
            .is_some_and(|glyph| matches!(glyph.data, skrifa::bitmap::BitmapData::Png(_)))
        && skrifa.color_glyphs().get(glyph_id).is_none()
        && !skrifa.svg().is_ok_and(|svg| svg.glyph_data(glyph_id).is_some())
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
pub fn glyph_frame(font: &FontInstance, glyph_id: GlyphId) -> Option<GlyphFrame> {
    let upem = Abs::pt(font.units_per_em());

    if let Some(frame) = draw_glyph(font, upem, glyph_id) {
        return Some(frame);
    }

    // Generate a fallback tofu if the glyph couldn't be drawn, unless it is
    // the space glyph. Then, an empty frame does the job. (This happens for
    // some rare CBDT fonts, which don't define a bitmap for the space, but
    // also don't have a glyf or CFF table.)
    let not_space = font.glyph_index(' ') != Some(glyph_id);
    not_space.then(|| draw_fallback_tofu(font, upem, glyph_id))
}

/// Tries to draw a glyph.
fn draw_glyph(font: &FontInstance, upem: Abs, glyph_id: GlyphId) -> Option<GlyphFrame> {
    let skrifa = font.skrifa();
    let kind = if let Some(raster) = skrifa
        .bitmap_strikes()
        .glyph_for_size(skrifa::instance::Size::unscaled(), glyph_id)
        .filter(|glyph| matches!(glyph.data, skrifa::bitmap::BitmapData::Png(_)))
    {
        draw_raster_glyph(font, upem, raster)
    } else if skrifa.color_glyphs().get(glyph_id).is_some() {
        draw_colr_glyph(font, glyph_id)
    } else if skrifa.svg().is_ok_and(|svg| svg.glyph_data(glyph_id).is_some()) {
        draw_svg_glyph(font, glyph_id)
    } else {
        None
    };

    kind.map(|kind| GlyphFrame { upem, item: kind })
}

/// Draws a fallback tofu box with the advance width of the glyph.
fn draw_fallback_tofu(font: &FontInstance, upem: Abs, glyph_id: GlyphId) -> GlyphFrame {
    let advance = font
        .glyph_metrics()
        .advance_width(glyph_id)
        .map(|advance| Abs::pt(advance as f64))
        .unwrap_or(upem / 3.0);
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
    raster: skrifa::bitmap::BitmapGlyph,
) -> Option<GlyphFrameItem> {
    let skrifa::bitmap::BitmapData::Png(data) = raster.data else { unreachable!() };
    let data = Bytes::new(data.to_vec());
    let image = Image::plain(RasterImage::plain(data, ExchangeFormat::Png).ok()?);

    let scale = upem / raster.ppem_x as f64;
    let image_width = scale * image.width();
    let image_height = scale * image.height();

    let x_offset = scale * raster.inner_bearing_x as f64;
    let y = match raster.placement_origin {
        skrifa::bitmap::Origin::TopLeft => raster.inner_bearing_y - raster.height as f32,
        skrifa::bitmap::Origin::BottomLeft => raster.inner_bearing_y,
    };
    let mut y_offset = scale * y as f64;
    // Apple Color emoji doesn't provide offset information (or at least
    // not in a way skrifa understands), so we artificially shift their
    // baseline to make it look good.
    if font.info().family.to_lowercase() == "apple color emoji" {
        // This factor is just taken from krilla.
        y_offset -= 0.128 * upem;
    }

    let position = Point::new(-x_offset, -(image_height + y_offset));
    let size = Size::new(image_width, image_height);
    Some(GlyphFrameItem::Image(position, image, size))
}

/// Draws a glyph from the COLR table into the frame.
fn draw_colr_glyph(font: &FontInstance, glyph_id: GlyphId) -> Option<GlyphFrameItem> {
    let svg_string = colr_glyph_to_svg(font, glyph_id)?;

    let head = font.skrifa().head().ok()?;
    let width = (head.x_max() - head.x_min()) as f64;
    let height = (head.y_max() - head.y_min()) as f64;
    let x_min = head.x_min() as f64;
    let y_max = head.y_max() as f64;

    let data = Bytes::from_string(svg_string);
    let image = Image::plain(SvgImage::new(data).ok()?);

    let position = Point::new(Abs::pt(x_min), Abs::pt(-y_max));
    let size = Size::new(Abs::pt(width), Abs::pt(height));
    Some(GlyphFrameItem::Image(position, image, size))
}

/// Convert a COLR glyph into an SVG file.
fn colr_glyph_to_svg(font: &FontInstance, glyph_id: GlyphId) -> Option<String> {
    let mut svg = XmlWriter::new(xmlwriter::Options::default());

    let head = font.skrifa().head().ok()?;
    let width = (head.x_max() - head.x_min()) as f64;
    let height = (head.y_max() - head.y_min()) as f64;
    let x_min = head.x_min() as f64;
    let y_max = head.y_max() as f64;
    let tx = -x_min;
    let ty = -y_max;

    svg.start_element("svg");
    svg.write_attribute("xmlns", "http://www.w3.org/2000/svg");
    svg.write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
    svg.write_attribute("width", &width);
    svg.write_attribute("height", &height);
    svg.write_attribute_fmt("viewBox", format_args!("0 0 {width} {height}"));

    let mut path_buf = String::with_capacity(256);
    let gradient_index = 1;
    let clip_path_index = 1;

    svg.start_element("g");
    svg.write_attribute_fmt(
        "transform",
        format_args!("matrix(1 0 0 -1 0 0) matrix(1 0 0 1 {tx} {ty})"),
    );

    let skrifa = font.skrifa();
    let location = font.location();
    let mut glyph_painter = GlyphPainter {
        font: skrifa,
        location,
        svg: &mut svg,
        path_buf: &mut path_buf,
        gradient_index,
        clip_path_index,
        foreground_color: Color { blue: 0, green: 0, red: 0, alpha: 255 },
        transform: Transform::default(),
        outline_transform: Transform::default(),
        transforms_stack: vec![Transform::default()],
        clip_stack: vec![],
    };

    skrifa
        .color_glyphs()
        .get(glyph_id)?
        .paint(location, &mut glyph_painter)
        .ok()?;
    svg.end_element();

    Some(svg.end_document())
}

/// Draws an SVG glyph in a frame.
fn draw_svg_glyph(font: &FontInstance, glyph_id: GlyphId) -> Option<GlyphFrameItem> {
    // TODO: Our current conversion of the SVG table works for Twitter Color Emoji,
    // but might not work for others. See also: https://github.com/RazrFalcon/resvg/pull/776
    let mut data = font.skrifa().svg().ok().and_then(|svg| svg.glyph_data(glyph_id))?;

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

struct ColrBuilder<'a> {
    path: &'a mut String,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl<'a> ColrBuilder<'a> {
    fn new(path: &'a mut String) -> Self {
        Self {
            path,
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
        }
    }

    fn add_point(&mut self, x: f32, y: f32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    /// Returns a conservative bounding box of the written path.
    /// It includes curve control points, so it can be larger than the exact
    /// bounding box, but never smaller.
    fn bounds(&self) -> Option<tiny_skia_path::Rect> {
        if self.min_x <= self.max_x && self.min_y <= self.max_y {
            tiny_skia_path::Rect::from_ltrb(
                self.min_x, self.min_y, self.max_x, self.max_y,
            )
        } else {
            None
        }
    }

    fn finish(&mut self) {
        if !self.path.is_empty() {
            self.path.pop(); // remove trailing space
        }
    }
}

impl skrifa::outline::OutlinePen for ColrBuilder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        self.add_point(x, y);
        write!(self.path, "M {x} {y} ").unwrap();
    }

    fn line_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        self.add_point(x, y);
        write!(self.path, "L {x} {y} ").unwrap();
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        use std::fmt::Write;
        self.add_point(x1, y1);
        self.add_point(x, y);
        write!(self.path, "Q {x1} {y1} {x} {y} ").unwrap();
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        use std::fmt::Write;
        self.add_point(x1, y1);
        self.add_point(x2, y2);
        self.add_point(x, y);
        write!(self.path, "C {x1} {y1} {x2} {y2} {x} {y} ").unwrap();
    }

    fn close(&mut self) {
        self.path.push_str("Z ");
    }
}

// NOTE: This is only a best-effort translation of COLR into SVG. It's not feature-complete
// and it's also not possible to make it feature-complete using just raw SVG features.
pub(crate) struct GlyphPainter<'a> {
    pub(crate) font: &'a skrifa::FontRef<'a>,
    /// The variation location to draw outlines at.
    pub(crate) location: skrifa::instance::LocationRef<'a>,
    pub(crate) svg: &'a mut xmlwriter::XmlWriter,
    pub(crate) path_buf: &'a mut String,
    pub(crate) gradient_index: usize,
    pub(crate) clip_path_index: usize,
    pub(crate) foreground_color: Color,
    pub(crate) transform: Transform,
    pub(crate) outline_transform: Transform,
    pub(crate) transforms_stack: Vec<Transform>,
    /// The bounding box of every active clip, in the root coordinate space.
    /// `None` means the clip is empty (or its bounds are unknown).
    pub(crate) clip_stack: Vec<Option<tiny_skia_path::Rect>>,
}

impl GlyphPainter<'_> {
    fn write_gradient_stops(&mut self, stops: &[ColorStop]) {
        for stop in stops {
            let color = self.palette_index_to_color(stop.palette_index, stop.alpha);
            self.svg.start_element("stop");
            self.svg.write_attribute("offset", &stop.offset);
            self.write_color_attribute("stop-color", color);
            let opacity = f32::from(color.alpha) / 255.0;
            self.svg.write_attribute("stop-opacity", &opacity);
            self.svg.end_element();
        }
    }

    fn write_color_attribute(&mut self, name: &str, color: Color) {
        self.svg.write_attribute_fmt(
            name,
            format_args!("rgb({}, {}, {})", color.red, color.green, color.blue),
        );
    }

    fn write_transform_attribute(&mut self, name: &str, ts: Transform) {
        if ts == Transform::default() {
            return;
        }

        self.svg.write_attribute_fmt(
            name,
            format_args!(
                "matrix({} {} {} {} {} {})",
                ts.xx, ts.yx, ts.xy, ts.yy, ts.dx, ts.dy
            ),
        );
    }

    fn write_spread_method_attribute(&mut self, extend: Extend) {
        self.svg.write_attribute(
            "spreadMethod",
            match extend {
                Extend::Pad => &"pad",
                Extend::Repeat => &"repeat",
                Extend::Reflect => &"reflect",
                Extend::Unknown => return,
            },
        );
    }

    fn paint_solid(&mut self, color: Color) {
        self.svg.start_element("path");
        self.write_color_attribute("fill", color);
        let opacity = f32::from(color.alpha) / 255.0;
        self.svg.write_attribute("fill-opacity", &opacity);
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_linear_gradient(
        &mut self,
        p0: skrifa::raw::types::Point<f32>,
        p1: skrifa::raw::types::Point<f32>,
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
        self.write_spread_method_attribute(extend);
        self.write_transform_attribute("gradientTransform", gradient_transform);
        self.write_gradient_stops(color_stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{gradient_id})"));
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_radial_gradient(
        &mut self,
        c0: skrifa::raw::types::Point<f32>,
        r0: f32,
        c1: skrifa::raw::types::Point<f32>,
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
        self.write_spread_method_attribute(extend);
        self.write_transform_attribute("gradientTransform", gradient_transform);
        self.write_gradient_stops(color_stops);
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{gradient_id})"));
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_sweep_gradient(
        &mut self,
        _c0: skrifa::raw::types::Point<f32>,
        _start_angle: f32,
        _end_angle: f32,
        _color_stops: &[ColorStop],
        _extend: Extend,
    ) {
    }
}

fn paint_transform(outline_transform: Transform, transform: Transform) -> Transform {
    let outline_transform = skrifa_to_tsp_transform(outline_transform);

    let gradient_transform = skrifa_to_tsp_transform(transform);

    let gradient_transform = outline_transform
        .invert()
        // In theory, we should error out. But the transform shouldn't ever be uninvertible, so let's ignore it.
        .unwrap_or_default()
        .pre_concat(gradient_transform);

    tsp_to_skrifa_transform(gradient_transform)
}

fn skrifa_to_tsp_transform(t: Transform) -> tiny_skia_path::Transform {
    tiny_skia_path::Transform::from_row(t.xx, t.yx, t.xy, t.yy, t.dx, t.dy)
}

fn tsp_to_skrifa_transform(t: tiny_skia_path::Transform) -> Transform {
    Transform {
        xx: t.sx,
        yx: t.ky,
        xy: t.kx,
        yy: t.sy,
        dx: t.tx,
        dy: t.ty,
    }
}

/// Returns the bounding box of `rect` transformed by `ts`.
fn map_rect(
    rect: tiny_skia_path::Rect,
    ts: tiny_skia_path::Transform,
) -> Option<tiny_skia_path::Rect> {
    let mut points = [
        tiny_skia_path::Point::from_xy(rect.left(), rect.top()),
        tiny_skia_path::Point::from_xy(rect.right(), rect.top()),
        tiny_skia_path::Point::from_xy(rect.left(), rect.bottom()),
        tiny_skia_path::Point::from_xy(rect.right(), rect.bottom()),
    ];
    ts.map_points(&mut points);
    let min_x = points.iter().map(|p| p.x).fold(f32::MAX, f32::min);
    let min_y = points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
    let max_y = points.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    tiny_skia_path::Rect::from_ltrb(min_x, min_y, max_x, max_y)
}

/// Returns the intersection of two rects, or `None` when they do not overlap.
fn intersect_rects(
    a: tiny_skia_path::Rect,
    b: tiny_skia_path::Rect,
) -> Option<tiny_skia_path::Rect> {
    tiny_skia_path::Rect::from_ltrb(
        a.left().max(b.left()),
        a.top().max(b.top()),
        a.right().min(b.right()),
        a.bottom().min(b.bottom()),
    )
}

impl GlyphPainter<'_> {
    fn clip_with_path(&mut self, path: &str) {
        let clip_id = format!("cp{}", self.clip_path_index);
        self.clip_path_index += 1;

        self.svg.start_element("clipPath");
        self.svg.write_attribute("id", &clip_id);
        self.svg.start_element("path");
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", &path);
        self.svg.end_element();
        self.svg.end_element();

        self.svg.start_element("g");
        self.svg
            .write_attribute_fmt("clip-path", format_args!("url(#{clip_id})"));
    }

    /// Outlines a glyph into `path_buf` at the current variation location
    /// (an empty path on failure), records the current transform as the
    /// outline transform and returns the outline's conservative bounding box
    /// in the glyph's local coordinate space.
    fn outline_glyph(&mut self, glyph_id: GlyphId) -> Option<tiny_skia_path::Rect> {
        self.path_buf.clear();

        let mut bounds = None;
        let outlined = if let Some(outliner) = self.font.outline_glyphs().get(glyph_id) {
            let mut builder = ColrBuilder::new(self.path_buf);
            let size = skrifa::instance::Size::unscaled();
            let ok = outliner
                .draw(
                    skrifa::outline::DrawSettings::unhinted(size, self.location)
                        .with_path_style(skrifa::outline::pen::PathStyle::HarfBuzz),
                    &mut builder,
                )
                .is_ok();
            if ok {
                builder.finish();
                bounds = builder.bounds();
            }
            ok
        } else {
            false
        };
        if !outlined {
            // A partial outline may have been written before a draw error.
            self.path_buf.clear();
        }

        // We have to write outline using the current transform.
        self.outline_transform = self.transform;

        bounds
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
        let lookup = || -> Option<Color> {
            // We always use the first palette. `ColorPalettes` handles
            // per-palette record offsets internally.
            let palettes = self.font.color_palettes();
            let palette = palettes.get(0)?;
            let color = palette.colors().get(palette_index as usize)?;
            Some(Color {
                red: color.red,
                green: color.green,
                blue: color.blue,
                alpha: color.alpha,
            })
        };

        let mut color = if palette_index == u16::MAX {
            self.foreground_color
        } else {
            lookup().unwrap_or(self.foreground_color)
        };

        // Multiply alpha
        color.alpha = ((color.alpha as f32) * alpha) as u8;

        color
    }
}

impl skrifa::color::ColorPainter for GlyphPainter<'_> {
    fn push_layer(&mut self, composite_mode: skrifa::color::CompositeMode) {
        self.svg.start_element("g");

        use skrifa::color::CompositeMode;
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

    fn push_transform(&mut self, transform: Transform) {
        self.transforms_stack.push(self.transform);
        self.transform *= transform;
    }

    fn fill_glyph(
        &mut self,
        glyph_id: GlyphId,
        brush_transform: Option<Transform>,
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
        let path = self.path_buf.clone();
        self.clip_with_path(&path);

        let root_bounds = bounds
            .and_then(|b| map_rect(b, skrifa_to_tsp_transform(self.outline_transform)));
        self.clip_stack.push(root_bounds);
    }

    fn push_clip_box(&mut self, clipbox: skrifa::raw::types::BoundingBox<f32>) {
        let x_min = clipbox.x_min;
        let x_max = clipbox.x_max;
        let y_min = clipbox.y_min;
        let y_max = clipbox.y_max;

        let clip_path = format!(
            "M {x_min} {y_min} L {x_max} {y_min} L {x_max} {y_max} L {x_min} {y_max} Z"
        );

        // The clip box is positioned by the current transform.
        self.outline_transform = self.transform;
        self.clip_with_path(&clip_path);

        let bounds = tiny_skia_path::Rect::from_ltrb(x_min, y_min, x_max, y_max)
            .and_then(|b| map_rect(b, skrifa_to_tsp_transform(self.outline_transform)));
        self.clip_stack.push(bounds);
    }

    fn pop_clip(&mut self) {
        self.svg.end_element();
        self.clip_stack.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        // A fill paints the intersection of all currently active clips.
        // Paint a rectangle covering that intersection and let the enclosing
        // clip groups shape it.

        let mut region: Option<tiny_skia_path::Rect> = None;
        for bounds in &self.clip_stack {
            // A clip with no (or unknown) bounds clips everything away.
            let Some(bounds) = bounds else { return };
            region = Some(match region {
                Some(region) => match intersect_rects(region, *bounds) {
                    Some(r) => r,
                    // An empty intersection - there is nothing to paint.
                    None => return,
                },
                None => *bounds,
            });
        }
        let Some(region) = region else { return };

        use std::fmt::Write;
        self.path_buf.clear();
        write!(
            self.path_buf,
            "M {} {} L {} {} L {} {} L {} {} Z",
            region.left(),
            region.top(),
            region.right(),
            region.top(),
            region.right(),
            region.bottom(),
            region.left(),
            region.bottom()
        )
        .unwrap();

        // The covering rectangle is in the root coordinate space.
        self.outline_transform = Transform::default();

        self.paint_brush(brush);
    }
}

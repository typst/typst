//! Utilities for color font handling

use std::io::Read;

use ttf_parser::{GlyphId, RgbaColor};
use usvg::tiny_skia_path;
use xmlwriter::XmlWriter;

use crate::layout::{Abs, Axes, Frame, FrameItem, Point, Size};
use crate::syntax::Span;
use crate::text::{Font, Glyph};
use crate::visualize::Image;

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
    } else if ttf.is_color_glyph(glyph_id) {
        draw_colr_glyph(&mut frame, upem, ttf, glyph_id);
    } else if ttf.glyph_svg_image(glyph_id).is_some() {
        draw_svg_glyph(&mut frame, upem, font, glyph_id);
    }

    frame
}

fn draw_colr_glyph(
    frame: &mut Frame,
    upem: Abs,
    ttf: &ttf_parser::Face,
    glyph_id: GlyphId,
) -> Option<()> {
    let mut svg = XmlWriter::new(xmlwriter::Options::default());

    let width = ttf.global_bounding_box().width() as f64;
    let height = ttf.global_bounding_box().height() as f64;
    let x_min = ttf.global_bounding_box().x_min as f64;
    let y_max = ttf.global_bounding_box().y_max as f64;
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

    let mut glyph_painter = GlyphPainter {
        face: ttf,
        svg: &mut svg,
        path_buf: &mut path_buf,
        gradient_index,
        clip_path_index,
        palette_index: 0,
        transform: ttf_parser::Transform::default(),
        outline_transform: ttf_parser::Transform::default(),
        transforms_stack: vec![ttf_parser::Transform::default()],
    };

    ttf.paint_color_glyph(glyph_id, 0, RgbaColor::new(0, 0, 0, 255), &mut glyph_painter)
        .unwrap();
    svg.end_element();

    let data = svg.end_document().into_bytes();

    let image = Image::new(
        data.into(),
        typst::visualize::ImageFormat::Vector(typst::visualize::VectorFormat::Svg),
        None,
    )
    .unwrap();

    let y_shift = Abs::pt(upem.to_pt() - y_max);
    let position = Point::new(Abs::pt(x_min), y_shift);
    let size = Axes::new(Abs::pt(width), Abs::pt(height));
    frame.push(position, FrameItem::Image(image, size, Span::detached()));

    Some(())
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

/// Draws an SVG glyph in a frame.
fn draw_svg_glyph(
    frame: &mut Frame,
    upem: Abs,
    font: &Font,
    glyph_id: GlyphId,
) -> Option<()> {
    // TODO: Our current conversion of the SVG table works for Twitter Color Emoji,
    // but might not work for others. See also: https://github.com/RazrFalcon/resvg/pull/776
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
    let tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;

    let bbox = tree.root().bounding_box();
    let width = bbox.width() as f64;
    let height = bbox.height() as f64;
    let left = bbox.left() as f64;
    let top = bbox.top() as f64;

    let mut data = tree.to_string(&usvg::WriteOptions::default());

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

struct ColrBuilder<'a>(&'a mut String);

impl ColrBuilder<'_> {
    fn finish(&mut self) {
        if !self.0.is_empty() {
            self.0.pop(); // remove trailing space
        }
    }
}

impl ttf_parser::OutlineBuilder for ColrBuilder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "M {x} {y} ").unwrap()
    }

    fn line_to(&mut self, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "L {x} {y} ").unwrap()
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "Q {x1} {y1} {x} {y} ").unwrap()
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        use std::fmt::Write;
        write!(self.0, "C {x1} {y1} {x2} {y2} {x} {y} ").unwrap()
    }

    fn close(&mut self) {
        self.0.push_str("Z ")
    }
}

// NOTE: This is only a best-effort translation of COLR into SVG. It's not feature-complete
// and it's also not possible to make it feature-complete using just raw SVG features.
pub(crate) struct GlyphPainter<'a> {
    pub(crate) face: &'a ttf_parser::Face<'a>,
    pub(crate) svg: &'a mut xmlwriter::XmlWriter,
    pub(crate) path_buf: &'a mut String,
    pub(crate) gradient_index: usize,
    pub(crate) clip_path_index: usize,
    pub(crate) palette_index: u16,
    pub(crate) transform: ttf_parser::Transform,
    pub(crate) outline_transform: ttf_parser::Transform,
    pub(crate) transforms_stack: Vec<ttf_parser::Transform>,
}

impl<'a> GlyphPainter<'a> {
    fn write_gradient_stops(&mut self, stops: ttf_parser::colr::GradientStopsIter) {
        for stop in stops {
            self.svg.start_element("stop");
            self.svg.write_attribute("offset", &stop.stop_offset);
            self.write_color_attribute("stop-color", stop.color);
            let opacity = f32::from(stop.color.alpha) / 255.0;
            self.svg.write_attribute("stop-opacity", &opacity);
            self.svg.end_element();
        }
    }

    fn write_color_attribute(&mut self, name: &str, color: ttf_parser::RgbaColor) {
        self.svg.write_attribute_fmt(
            name,
            format_args!("rgb({}, {}, {})", color.red, color.green, color.blue),
        );
    }

    fn write_transform_attribute(&mut self, name: &str, ts: ttf_parser::Transform) {
        if ts.is_default() {
            return;
        }

        self.svg.write_attribute_fmt(
            name,
            format_args!("matrix({} {} {} {} {} {})", ts.a, ts.b, ts.c, ts.d, ts.e, ts.f),
        );
    }

    fn write_spread_method_attribute(
        &mut self,
        extend: ttf_parser::colr::GradientExtend,
    ) {
        self.svg.write_attribute(
            "spreadMethod",
            match extend {
                ttf_parser::colr::GradientExtend::Pad => &"pad",
                ttf_parser::colr::GradientExtend::Repeat => &"repeat",
                ttf_parser::colr::GradientExtend::Reflect => &"reflect",
            },
        );
    }

    fn paint_solid(&mut self, color: ttf_parser::RgbaColor) {
        self.svg.start_element("path");
        self.write_color_attribute("fill", color);
        let opacity = f32::from(color.alpha) / 255.0;
        self.svg.write_attribute("fill-opacity", &opacity);
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_linear_gradient(&mut self, gradient: ttf_parser::colr::LinearGradient<'a>) {
        let gradient_id = format!("lg{}", self.gradient_index);
        self.gradient_index += 1;

        let gradient_transform = paint_transform(self.outline_transform, self.transform);

        // TODO: We ignore x2, y2. Have to apply them somehow.
        // TODO: The way spreadMode works in ttf and svg is a bit different. In SVG, the spreadMode
        // will always be applied based on x1/y1 and x2/y2. However, in TTF the spreadMode will
        // be applied from the first/last stop. So if we have a gradient with x1=0 x2=1, and
        // a stop at x=0.4 and x=0.6, then in SVG we will always see a padding, while in ttf
        // we will see the actual spreadMode. We need to account for that somehow.
        self.svg.start_element("linearGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("x1", &gradient.x0);
        self.svg.write_attribute("y1", &gradient.y0);
        self.svg.write_attribute("x2", &gradient.x1);
        self.svg.write_attribute("y2", &gradient.y1);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        self.write_spread_method_attribute(gradient.extend);
        self.write_transform_attribute("gradientTransform", gradient_transform);
        self.write_gradient_stops(
            gradient.stops(self.palette_index, self.face.variation_coordinates()),
        );
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{gradient_id})"));
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_radial_gradient(&mut self, gradient: ttf_parser::colr::RadialGradient<'a>) {
        let gradient_id = format!("rg{}", self.gradient_index);
        self.gradient_index += 1;

        let gradient_transform = paint_transform(self.outline_transform, self.transform);

        self.svg.start_element("radialGradient");
        self.svg.write_attribute("id", &gradient_id);
        self.svg.write_attribute("cx", &gradient.x1);
        self.svg.write_attribute("cy", &gradient.y1);
        self.svg.write_attribute("r", &gradient.r1);
        self.svg.write_attribute("fr", &gradient.r0);
        self.svg.write_attribute("fx", &gradient.x0);
        self.svg.write_attribute("fy", &gradient.y0);
        self.svg.write_attribute("gradientUnits", &"userSpaceOnUse");
        self.write_spread_method_attribute(gradient.extend);
        self.write_transform_attribute("gradientTransform", gradient_transform);
        self.write_gradient_stops(
            gradient.stops(self.palette_index, self.face.variation_coordinates()),
        );
        self.svg.end_element();

        self.svg.start_element("path");
        self.svg
            .write_attribute_fmt("fill", format_args!("url(#{gradient_id})"));
        self.write_transform_attribute("transform", self.outline_transform);
        self.svg.write_attribute("d", self.path_buf);
        self.svg.end_element();
    }

    fn paint_sweep_gradient(&mut self, _: ttf_parser::colr::SweepGradient<'a>) {}
}

fn paint_transform(
    outline_transform: ttf_parser::Transform,
    transform: ttf_parser::Transform,
) -> ttf_parser::Transform {
    let outline_transform = tiny_skia_path::Transform::from_row(
        outline_transform.a,
        outline_transform.b,
        outline_transform.c,
        outline_transform.d,
        outline_transform.e,
        outline_transform.f,
    );

    let gradient_transform = tiny_skia_path::Transform::from_row(
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.e,
        transform.f,
    );

    let gradient_transform = outline_transform
        .invert()
        // In theory, we should error out. But the transform shouldn't ever be uninvertible, so let's ignore it.
        .unwrap_or_default()
        .pre_concat(gradient_transform);

    ttf_parser::Transform {
        a: gradient_transform.sx,
        b: gradient_transform.ky,
        c: gradient_transform.kx,
        d: gradient_transform.sy,
        e: gradient_transform.tx,
        f: gradient_transform.ty,
    }
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
}

impl<'a> ttf_parser::colr::Painter<'a> for GlyphPainter<'a> {
    fn outline_glyph(&mut self, glyph_id: ttf_parser::GlyphId) {
        self.path_buf.clear();
        let mut builder = ColrBuilder(self.path_buf);
        match self.face.outline_glyph(glyph_id, &mut builder) {
            Some(v) => v,
            None => return,
        };
        builder.finish();

        // We have to write outline using the current transform.
        self.outline_transform = self.transform;
    }

    fn push_layer(&mut self, mode: ttf_parser::colr::CompositeMode) {
        self.svg.start_element("g");

        use ttf_parser::colr::CompositeMode;
        // TODO: Need to figure out how to represent the other blend modes
        // in SVG.
        let mode = match mode {
            CompositeMode::SourceOver => "normal",
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
            CompositeMode::Hue => "hue",
            CompositeMode::Saturation => "saturation",
            CompositeMode::Color => "color",
            CompositeMode::Luminosity => "luminosity",
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

    fn push_transform(&mut self, transform: ttf_parser::Transform) {
        self.transforms_stack.push(self.transform);
        self.transform = ttf_parser::Transform::combine(self.transform, transform);
    }

    fn paint(&mut self, paint: ttf_parser::colr::Paint<'a>) {
        match paint {
            ttf_parser::colr::Paint::Solid(color) => self.paint_solid(color),
            ttf_parser::colr::Paint::LinearGradient(lg) => self.paint_linear_gradient(lg),
            ttf_parser::colr::Paint::RadialGradient(rg) => self.paint_radial_gradient(rg),
            ttf_parser::colr::Paint::SweepGradient(sg) => self.paint_sweep_gradient(sg),
        }
    }

    fn pop_transform(&mut self) {
        if let Some(ts) = self.transforms_stack.pop() {
            self.transform = ts
        }
    }

    fn push_clip(&mut self) {
        self.clip_with_path(&self.path_buf.clone());
    }

    fn pop_clip(&mut self) {
        self.svg.end_element();
    }

    fn push_clip_box(&mut self, clipbox: ttf_parser::colr::ClipBox) {
        let x_min = clipbox.x_min;
        let x_max = clipbox.x_max;
        let y_min = clipbox.y_min;
        let y_max = clipbox.y_max;

        let clip_path = format!(
            "M {x_min} {y_min} L {x_max} {y_min} L {x_max} {y_max} L {x_min} {y_max} Z"
        );

        self.clip_with_path(&clip_path);
    }
}

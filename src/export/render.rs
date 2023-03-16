//! Rendering into raster images.

use std::io::Read;
use std::sync::Arc;

use image::imageops::FilterType;
use image::{GenericImageView, Rgba};
use tiny_skia as sk;
use ttf_parser::{GlyphId, OutlineBuilder};
use usvg::FitTo;

use crate::doc::{Element, Frame, Group, Meta, Text};
use crate::geom::{
    self, Abs, Color, Geometry, Paint, PathElement, Shape, Size, Stroke, Transform,
};
use crate::image::{DecodedImage, Image};

/// Export a frame into a raster image.
///
/// This renders the frame at the given number of pixels per point and returns
/// the resulting `tiny-skia` pixel buffer.
pub fn render(frame: &Frame, pixel_per_pt: f32, fill: Color) -> sk::Pixmap {
    let size = frame.size();
    let pxw = (pixel_per_pt * size.x.to_f32()).round().max(1.0) as u32;
    let pxh = (pixel_per_pt * size.y.to_f32()).round().max(1.0) as u32;

    let mut canvas = sk::Pixmap::new(pxw, pxh).unwrap();
    canvas.fill(fill.into());

    let ts = sk::Transform::from_scale(pixel_per_pt, pixel_per_pt);
    render_frame(&mut canvas, ts, None, frame);

    canvas
}

/// Render all elements in a frame into the canvas.
fn render_frame(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    frame: &Frame,
) {
    for (pos, element) in frame.elements() {
        let x = pos.x.to_f32();
        let y = pos.y.to_f32();
        let ts = ts.pre_translate(x, y);

        match element {
            Element::Group(group) => {
                render_group(canvas, ts, mask, group);
            }
            Element::Text(text) => {
                render_text(canvas, ts, mask, text);
            }
            Element::Shape(shape, _) => {
                render_shape(canvas, ts, mask, shape);
            }
            Element::Image(image, size, _) => {
                render_image(canvas, ts, mask, image, *size);
            }
            Element::Meta(meta, _) => match meta {
                Meta::Link(_) => {}
                Meta::Node(_) => {}
                Meta::Hide => {}
            },
        }
    }
}

/// Render a group frame with optional transform and clipping into the canvas.
fn render_group(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    group: &Group,
) {
    let ts = ts.pre_concat(group.transform.into());

    let mut mask = mask;
    let mut storage;
    if group.clips {
        let size = group.frame.size();
        let w = size.x.to_f32();
        let h = size.y.to_f32();
        if let Some(path) = sk::Rect::from_xywh(0.0, 0.0, w, h)
            .map(sk::PathBuilder::from_rect)
            .and_then(|path| path.transform(ts))
        {
            let result = if let Some(mask) = mask {
                storage = mask.clone();
                storage.intersect_path(&path, sk::FillRule::default(), false)
            } else {
                let pxw = canvas.width();
                let pxh = canvas.height();
                storage = sk::ClipMask::new();
                storage.set_path(pxw, pxh, &path, sk::FillRule::default(), false)
            };

            // Clipping fails if clipping rect is empty. In that case we just
            // clip everything by returning.
            if result.is_none() {
                return;
            }

            mask = Some(&storage);
        }
    }

    render_frame(canvas, ts, mask, &group.frame);
}

/// Render a text run into the canvas.
fn render_text(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    text: &Text,
) {
    let mut x = 0.0;
    for glyph in &text.glyphs {
        let id = GlyphId(glyph.id);
        let offset = x + glyph.x_offset.at(text.size).to_f32();
        let ts = ts.pre_translate(offset, 0.0);

        render_svg_glyph(canvas, ts, mask, text, id)
            .or_else(|| render_bitmap_glyph(canvas, ts, mask, text, id))
            .or_else(|| render_outline_glyph(canvas, ts, mask, text, id));

        x += glyph.x_advance.at(text.size).to_f32();
    }
}

/// Render an SVG glyph into the canvas.
fn render_svg_glyph(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    _: Option<&sk::ClipMask>,
    text: &Text,
    id: GlyphId,
) -> Option<()> {
    let mut data = text.font.ttf().glyph_svg_image(id)?;

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
    let tree = usvg::Tree::from_xmltree(&document, &opts.to_ref()).ok()?;
    let view_box = tree.svg_node().view_box.rect;

    // If there's no viewbox defined, use the em square for our scale
    // transformation ...
    let upem = text.font.units_per_em() as f32;
    let (mut width, mut height) = (upem, upem);

    // ... but if there's a viewbox or width, use that.
    if root.has_attribute("viewBox") || root.has_attribute("width") {
        width = view_box.width() as f32;
    }

    // Same as for width.
    if root.has_attribute("viewBox") || root.has_attribute("height") {
        height = view_box.height() as f32;
    }

    // FIXME: This doesn't respect the clipping mask.
    let size = text.size.to_f32();
    let ts = ts.pre_scale(size / width, size / height);
    resvg::render(&tree, FitTo::Original, ts, canvas.as_mut())
}

/// Render a bitmap glyph into the canvas.
fn render_bitmap_glyph(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    text: &Text,
    id: GlyphId,
) -> Option<()> {
    let size = text.size.to_f32();
    let ppem = size * ts.sy;
    let raster = text.font.ttf().glyph_raster_image(id, ppem as u16)?;
    let image = Image::new(raster.data.into(), raster.format.into()).ok()?;

    // FIXME: Vertical alignment isn't quite right for Apple Color Emoji,
    // and maybe also for Noto Color Emoji. And: Is the size calculation
    // correct?
    let h = text.size;
    let w = (image.width() as f64 / image.height() as f64) * h;
    let dx = (raster.x as f32) / (image.width() as f32) * size;
    let dy = (raster.y as f32) / (image.height() as f32) * size;
    let ts = ts.pre_translate(dx, -size - dy);
    render_image(canvas, ts, mask, &image, Size::new(w, h))
}

/// Render an outline glyph into the canvas. This is the "normal" case.
fn render_outline_glyph(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    text: &Text,
    id: GlyphId,
) -> Option<()> {
    let ppem = text.size.to_f32() * ts.sy;

    // Render a glyph directly as a path. This only happens when the fast glyph
    // rasterization can't be used due to very large text size or weird
    // scale/skewing transforms.
    if ppem > 100.0 || ts.kx != 0.0 || ts.ky != 0.0 || ts.sx != ts.sy {
        let path = {
            let mut builder = WrappedPathBuilder(sk::PathBuilder::new());
            text.font.ttf().outline_glyph(id, &mut builder)?;
            builder.0.finish()?
        };

        let paint = text.fill.into();
        let rule = sk::FillRule::default();

        // Flip vertically because font design coordinate
        // system is Y-up.
        let scale = text.size.to_f32() / text.font.units_per_em() as f32;
        let ts = ts.pre_scale(scale, -scale);
        canvas.fill_path(&path, &paint, rule, ts, mask)?;
        return Some(());
    }

    // Rasterize the glyph with `pixglyph`.
    // Try to retrieve a prepared glyph or prepare it from scratch if it
    // doesn't exist, yet.
    let glyph = pixglyph::Glyph::load(text.font.ttf(), id)?;
    let bitmap = glyph.rasterize(ts.tx, ts.ty, ppem);
    let cw = canvas.width() as i32;
    let ch = canvas.height() as i32;
    let mw = bitmap.width as i32;
    let mh = bitmap.height as i32;

    // Determine the pixel bounding box that we actually need to draw.
    let left = bitmap.left;
    let right = left + mw;
    let top = bitmap.top;
    let bottom = top + mh;

    // Premultiply the text color.
    let Paint::Solid(color) = text.fill;
    let c = color.to_rgba();
    let color = sk::ColorU8::from_rgba(c.r, c.g, c.b, 255).premultiply().get();

    // Blend the glyph bitmap with the existing pixels on the canvas.
    // FIXME: This doesn't respect the clipping mask.
    let pixels = bytemuck::cast_slice_mut::<u8, u32>(canvas.data_mut());
    for x in left.clamp(0, cw)..right.clamp(0, cw) {
        for y in top.clamp(0, ch)..bottom.clamp(0, ch) {
            let ai = ((y - top) * mw + (x - left)) as usize;
            let cov = bitmap.coverage[ai];
            if cov == 0 {
                continue;
            }

            let pi = (y * cw + x) as usize;
            if cov == 255 {
                pixels[pi] = color;
                continue;
            }

            let applied = alpha_mul(color, cov as u32);
            pixels[pi] = blend_src_over(applied, pixels[pi]);
        }
    }

    Some(())
}

/// Render a geometrical shape into the canvas.
fn render_shape(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    shape: &Shape,
) -> Option<()> {
    let path = match shape.geometry {
        Geometry::Line(target) => {
            let mut builder = sk::PathBuilder::new();
            builder.line_to(target.x.to_f32(), target.y.to_f32());
            builder.finish()?
        }
        Geometry::Rect(size) => {
            let w = size.x.to_f32();
            let h = size.y.to_f32();
            let rect = sk::Rect::from_xywh(0.0, 0.0, w, h)?;
            sk::PathBuilder::from_rect(rect)
        }
        Geometry::Path(ref path) => convert_path(path)?,
    };

    if let Some(fill) = shape.fill {
        let mut paint: sk::Paint = fill.into();
        if matches!(shape.geometry, Geometry::Rect(_)) {
            paint.anti_alias = false;
        }

        let rule = sk::FillRule::default();
        canvas.fill_path(&path, &paint, rule, ts, mask);
    }

    if let Some(Stroke { paint, thickness }) = shape.stroke {
        let paint = paint.into();
        let stroke = sk::Stroke { width: thickness.to_f32(), ..Default::default() };
        canvas.stroke_path(&path, &paint, &stroke, ts, mask);
    }

    Some(())
}

/// Convert a Typst path into a tiny-skia path.
fn convert_path(path: &geom::Path) -> Option<sk::Path> {
    let mut builder = sk::PathBuilder::new();
    for elem in &path.0 {
        match elem {
            PathElement::MoveTo(p) => {
                builder.move_to(p.x.to_f32(), p.y.to_f32());
            }
            PathElement::LineTo(p) => {
                builder.line_to(p.x.to_f32(), p.y.to_f32());
            }
            PathElement::CubicTo(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x.to_f32(),
                    p1.y.to_f32(),
                    p2.x.to_f32(),
                    p2.y.to_f32(),
                    p3.x.to_f32(),
                    p3.y.to_f32(),
                );
            }
            PathElement::ClosePath => {
                builder.close();
            }
        };
    }
    builder.finish()
}

/// Render a raster or SVG image into the canvas.
fn render_image(
    canvas: &mut sk::Pixmap,
    ts: sk::Transform,
    mask: Option<&sk::ClipMask>,
    image: &Image,
    size: Size,
) -> Option<()> {
    let view_width = size.x.to_f32();
    let view_height = size.y.to_f32();

    let aspect = (image.width() as f32) / (image.height() as f32);
    let scale = ts.sx.max(ts.sy);
    let w = (scale * view_width.max(aspect * view_height)).ceil() as u32;
    let h = ((w as f32) / aspect).ceil() as u32;

    let pixmap = scaled_texture(image, w, h)?;
    let scale_x = view_width / pixmap.width() as f32;
    let scale_y = view_height / pixmap.height() as f32;

    let paint = sk::Paint {
        shader: sk::Pattern::new(
            (*pixmap).as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            1.0,
            sk::Transform::from_scale(scale_x, scale_y),
        ),
        ..Default::default()
    };

    let rect = sk::Rect::from_xywh(0.0, 0.0, view_width, view_height)?;
    canvas.fill_rect(rect, &paint, ts, mask);

    Some(())
}

/// Prepare a texture for an image at a scaled size.
#[comemo::memoize]
fn scaled_texture(image: &Image, w: u32, h: u32) -> Option<Arc<sk::Pixmap>> {
    let mut pixmap = sk::Pixmap::new(w, h)?;
    match image.decode().unwrap().as_ref() {
        DecodedImage::Raster(dynamic, _) => {
            let downscale = w < image.width();
            let filter =
                if downscale { FilterType::Lanczos3 } else { FilterType::CatmullRom };
            let buf = dynamic.resize(w, h, filter);
            for ((_, _, src), dest) in buf.pixels().zip(pixmap.pixels_mut()) {
                let Rgba([r, g, b, a]) = src;
                *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
            }
        }
        DecodedImage::Svg(tree) => {
            resvg::render(
                tree,
                FitTo::Size(w, h),
                sk::Transform::identity(),
                pixmap.as_mut(),
            )?;
        }
    }
    Some(Arc::new(pixmap))
}

impl From<Transform> for sk::Transform {
    fn from(transform: Transform) -> Self {
        let Transform { sx, ky, kx, sy, tx, ty } = transform;
        sk::Transform::from_row(
            sx.get() as _,
            ky.get() as _,
            kx.get() as _,
            sy.get() as _,
            tx.to_f32(),
            ty.to_f32(),
        )
    }
}

impl From<Paint> for sk::Paint<'static> {
    fn from(paint: Paint) -> Self {
        let mut sk_paint = sk::Paint::default();
        let Paint::Solid(color) = paint;
        sk_paint.set_color(color.into());
        sk_paint.anti_alias = true;
        sk_paint
    }
}

impl From<Color> for sk::Color {
    fn from(color: Color) -> Self {
        let c = color.to_rgba();
        sk::Color::from_rgba8(c.r, c.g, c.b, c.a)
    }
}

/// Allows to build tiny-skia paths from glyph outlines.
struct WrappedPathBuilder(sk::PathBuilder);

impl OutlineBuilder for WrappedPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.0.close();
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

// Alpha multiplication and blending are ported from:
// https://skia.googlesource.com/skia/+/refs/heads/main/include/core/SkColorPriv.h

/// Blends two premulitplied, packed 32-bit RGBA colors. Alpha channel must be
/// in the 8 high bits.
fn blend_src_over(src: u32, dst: u32) -> u32 {
    src + alpha_mul(dst, 256 - (src >> 24))
}

/// Alpha multiply a color.
fn alpha_mul(color: u32, scale: u32) -> u32 {
    let mask = 0xff00ff;
    let rb = ((color & mask) * scale) >> 8;
    let ag = ((color >> 8) & mask) * scale;
    (rb & mask) | (ag & !mask)
}

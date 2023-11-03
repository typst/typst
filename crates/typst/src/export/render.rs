//! Rendering into raster images.

use std::io::Read;
use std::sync::Arc;

use image::imageops::FilterType;
use image::{GenericImageView, Rgba};
use pixglyph::Bitmap;
use resvg::tiny_skia::IntRect;
use tiny_skia as sk;
use ttf_parser::{GlyphId, OutlineBuilder};
use usvg::{NodeExt, TreeParsing};

use crate::doc::{Frame, FrameItem, FrameKind, GroupItem, Meta, TextItem};
use crate::font::Font;
use crate::geom::{
    self, Abs, Axes, Color, FixedStroke, Geometry, Gradient, LineCap, LineJoin, Paint,
    PathItem, Point, Ratio, Relative, Shape, Size, Transform,
};
use crate::image::{Image, ImageKind, RasterFormat};

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
    render_frame(&mut canvas, State::new(size, ts, pixel_per_pt), frame);

    canvas
}

/// Export multiple frames into a single raster image.
///
/// The padding will be added around and between the individual frames.
pub fn render_merged(
    frames: &[Frame],
    pixel_per_pt: f32,
    frame_fill: Color,
    padding: Abs,
    padding_fill: Color,
) -> sk::Pixmap {
    let pixmaps: Vec<_> = frames
        .iter()
        .map(|frame| typst::export::render(frame, pixel_per_pt, frame_fill))
        .collect();

    let padding = (pixel_per_pt * padding.to_f32()).round() as u32;
    let pxw =
        2 * padding + pixmaps.iter().map(sk::Pixmap::width).max().unwrap_or_default();
    let pxh =
        padding + pixmaps.iter().map(|pixmap| pixmap.height() + padding).sum::<u32>();

    let mut canvas = sk::Pixmap::new(pxw, pxh).unwrap();
    canvas.fill(padding_fill.into());

    let [x, mut y] = [padding; 2];
    for pixmap in pixmaps {
        canvas.draw_pixmap(
            x as i32,
            y as i32,
            pixmap.as_ref(),
            &sk::PixmapPaint::default(),
            sk::Transform::identity(),
            None,
        );

        y += pixmap.height() + padding;
    }

    canvas
}

/// Additional metadata carried through the rendering process.
#[derive(Clone, Copy, Default)]
struct State<'a> {
    /// The transform of the current item.
    transform: sk::Transform,
    /// The transform of the first hard frame in the hierarchy.
    container_transform: sk::Transform,
    /// The mask of the current item.
    mask: Option<&'a sk::Mask>,
    /// The pixel per point ratio.
    pixel_per_pt: f32,
    /// The size of the first hard frame in the hierarchy.
    size: Size,
}

impl<'a> State<'a> {
    fn new(size: Size, transform: sk::Transform, pixel_per_pt: f32) -> Self {
        Self {
            size,
            transform,
            container_transform: transform,
            pixel_per_pt,
            ..Default::default()
        }
    }

    /// Pre translate the current item's transform.
    fn pre_translate(self, pos: Point) -> Self {
        Self {
            transform: self.transform.pre_translate(pos.x.to_f32(), pos.y.to_f32()),
            ..self
        }
    }

    /// Pre concat the current item's transform.
    fn pre_concat(self, transform: sk::Transform) -> Self {
        Self {
            transform: self.transform.pre_concat(transform),
            ..self
        }
    }

    /// Sets the current mask.
    fn with_mask(self, mask: Option<&sk::Mask>) -> State<'_> {
        // Ensure that we're using the parent's mask if we don't have one.
        if mask.is_some() {
            State { mask, ..self }
        } else {
            State { mask: None, ..self }
        }
    }

    /// Sets the size of the first hard frame in the hierarchy.
    fn with_size(self, size: Size) -> Self {
        Self { size, ..self }
    }

    /// Pre concat the container's transform.
    fn pre_concat_container(self, transform: sk::Transform) -> Self {
        Self {
            container_transform: self.container_transform.pre_concat(transform),
            ..self
        }
    }
}

/// Render a frame into the canvas.
fn render_frame(canvas: &mut sk::Pixmap, state: State, frame: &Frame) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                render_group(canvas, state, *pos, group);
            }
            FrameItem::Text(text) => {
                render_text(canvas, state.pre_translate(*pos), text);
            }
            FrameItem::Shape(shape, _) => {
                render_shape(canvas, state.pre_translate(*pos), shape);
            }
            FrameItem::Image(image, size, _) => {
                render_image(canvas, state.pre_translate(*pos), image, *size);
            }
            FrameItem::Meta(meta, _) => match meta {
                Meta::Link(_) => {}
                Meta::Elem(_) => {}
                Meta::PageNumbering(_) => {}
                Meta::PdfPageLabel(_) => {}
                Meta::Hide => {}
            },
        }
    }
}

/// Render a group frame with optional transform and clipping into the canvas.
fn render_group(canvas: &mut sk::Pixmap, state: State, pos: Point, group: &GroupItem) {
    let state = match group.frame.kind() {
        FrameKind::Soft => state.pre_translate(pos).pre_concat(group.transform.into()),
        FrameKind::Hard => state
            .pre_translate(pos)
            .pre_concat(group.transform.into())
            .pre_concat_container(
                state
                    .transform
                    .post_concat(state.container_transform.invert().unwrap()),
            )
            .pre_concat_container(Transform::translate(pos.x, pos.y).into())
            .pre_concat_container(group.transform.into())
            .with_size(group.frame.size()),
    };

    let mut mask = state.mask;
    let storage;
    if let Some(clip_path) = group.clip_path.as_ref() {
        if let Some(path) =
            convert_path(clip_path).and_then(|path| path.transform(state.transform))
        {
            if let Some(mask) = mask {
                let mut mask = mask.clone();
                mask.intersect_path(
                    &path,
                    sk::FillRule::default(),
                    false,
                    sk::Transform::default(),
                );
                storage = mask;
            } else {
                let pxw = canvas.width();
                let pxh = canvas.height();
                let Some(mut mask) = sk::Mask::new(pxw, pxh) else {
                    // Fails if clipping rect is empty. In that case we just
                    // clip everything by returning.
                    return;
                };

                mask.fill_path(
                    &path,
                    sk::FillRule::default(),
                    false,
                    sk::Transform::default(),
                );
                storage = mask;
            };

            mask = Some(&storage);
        }
    }

    render_frame(canvas, state.with_mask(mask), &group.frame);
}

/// Render a text run into the canvas.
fn render_text(canvas: &mut sk::Pixmap, state: State, text: &TextItem) {
    let mut x = 0.0;
    for glyph in &text.glyphs {
        let id = GlyphId(glyph.id);
        let offset = x + glyph.x_offset.at(text.size).to_f32();
        let state = state.pre_translate(Point::new(Abs::raw(offset as _), Abs::raw(0.0)));

        render_svg_glyph(canvas, state, text, id)
            .or_else(|| render_bitmap_glyph(canvas, state, text, id))
            .or_else(|| render_outline_glyph(canvas, state, text, id));

        x += glyph.x_advance.at(text.size).to_f32();
    }
}

/// Render an SVG glyph into the canvas.
fn render_svg_glyph(
    canvas: &mut sk::Pixmap,
    state: State,
    text: &TextItem,
    id: GlyphId,
) -> Option<()> {
    let ts = &state.transform;
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
    let usvg_tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;
    let tree = resvg::Tree::from_usvg(&usvg_tree);
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

    let size = text.size.to_f32();
    let ts = ts.pre_scale(size / width, size / height);

    // Compute the space we need to draw our glyph.
    // See https://github.com/RazrFalcon/resvg/issues/602 for why
    // using the svg size is problematic here.
    let mut bbox = usvg::BBox::default();
    for node in usvg_tree.root.descendants() {
        if let Some(rect) = node.calculate_bbox() {
            bbox = bbox.expand(rect);
        }
    }

    // Compute the bbox after the transform is applied.
    // We add a nice 5px border along the bounding box to
    // be on the safe size. We also compute the intersection
    // with the canvas rectangle
    let bbox = bbox.transform(ts)?.to_rect()?.round_out()?;
    let bbox = IntRect::from_xywh(
        bbox.left() - 5,
        bbox.y() - 5,
        bbox.width() + 10,
        bbox.height() + 10,
    )?;

    let mut pixmap = sk::Pixmap::new(bbox.width(), bbox.height())?;

    // We offset our transform so that the pixmap starts at the edge of the bbox.
    let ts = ts.post_translate(-bbox.left() as f32, -bbox.top() as f32);
    tree.render(ts, &mut pixmap.as_mut());

    canvas.draw_pixmap(
        bbox.left(),
        bbox.top(),
        pixmap.as_ref(),
        &sk::PixmapPaint::default(),
        sk::Transform::identity(),
        state.mask,
    );

    Some(())
}

/// Render a bitmap glyph into the canvas.
fn render_bitmap_glyph(
    canvas: &mut sk::Pixmap,
    state: State,
    text: &TextItem,
    id: GlyphId,
) -> Option<()> {
    let ts = state.transform;
    let size = text.size.to_f32();
    let ppem = size * ts.sy;
    let raster = text.font.ttf().glyph_raster_image(id, ppem as u16)?;
    if raster.format != ttf_parser::RasterImageFormat::PNG {
        return None;
    }
    let image = Image::new(raster.data.into(), RasterFormat::Png.into(), None).ok()?;

    // FIXME: Vertical alignment isn't quite right for Apple Color Emoji,
    // and maybe also for Noto Color Emoji. And: Is the size calculation
    // correct?
    let h = text.size;
    let w = (image.width() as f64 / image.height() as f64) * h;
    let dx = (raster.x as f32) / (image.width() as f32) * size;
    let dy = (raster.y as f32) / (image.height() as f32) * size;
    render_image(
        canvas,
        state.pre_translate(Point::new(Abs::raw(dx as _), Abs::raw((-size - dy) as _))),
        &image,
        Size::new(w, h),
    )
}

/// Render an outline glyph into the canvas. This is the "normal" case.
fn render_outline_glyph(
    canvas: &mut sk::Pixmap,
    state: State,
    text: &TextItem,
    id: GlyphId,
) -> Option<()> {
    let ts = &state.transform;
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

        let scale = text.size.to_f32() / text.font.units_per_em() as f32;

        let mut pixmap = None;
        let paint = to_sk_paint(
            &text.fill,
            state.pre_concat(sk::Transform::from_scale(scale, -scale)),
            Size::zero(),
            true,
            None,
            &mut pixmap,
            None,
        );

        let rule = sk::FillRule::default();

        // Flip vertically because font design coordinate
        // system is Y-up.
        let ts = ts.pre_scale(scale, -scale);
        canvas.fill_path(&path, &paint, rule, ts, state.mask);
        return Some(());
    }

    // Rasterize the glyph with `pixglyph`.
    #[comemo::memoize]
    fn rasterize(
        font: &Font,
        id: GlyphId,
        x: u32,
        y: u32,
        size: u32,
    ) -> Option<Arc<Bitmap>> {
        let glyph = pixglyph::Glyph::load(font.ttf(), id)?;
        Some(Arc::new(glyph.rasterize(
            f32::from_bits(x),
            f32::from_bits(y),
            f32::from_bits(size),
        )))
    }

    // Try to retrieve a prepared glyph or prepare it from scratch if it
    // doesn't exist, yet.
    let bitmap =
        rasterize(&text.font, id, ts.tx.to_bits(), ts.ty.to_bits(), ppem.to_bits())?;
    match &text.fill {
        Paint::Gradient(gradient) => {
            let sampler = GradientSampler::new(gradient, &state, Size::zero(), true);
            write_bitmap(canvas, &bitmap, &state, sampler)?;
        }
        Paint::Solid(color) => {
            write_bitmap(canvas, &bitmap, &state, *color)?;
        }
    }

    Some(())
}

fn write_bitmap<S: PaintSampler>(
    canvas: &mut sk::Pixmap,
    bitmap: &Bitmap,
    state: &State,
    sampler: S,
) -> Option<()> {
    // If we have a clip mask we first render to a pixmap that we then blend
    // with our canvas
    if state.mask.is_some() {
        let mw = bitmap.width;
        let mh = bitmap.height;

        // Pad the pixmap with 1 pixel in each dimension so that we do
        // not get any problem with floating point errors along their border
        let mut pixmap = sk::Pixmap::new(mw + 2, mh + 2)?;
        for x in 0..mw {
            for y in 0..mh {
                let alpha = bitmap.coverage[(y * mw + x) as usize];
                let color: sk::ColorU8 = sampler.sample((x, y)).into();

                pixmap.pixels_mut()[((y + 1) * (mw + 2) + (x + 1)) as usize] =
                    sk::ColorU8::from_rgba(
                        color.red(),
                        color.green(),
                        color.blue(),
                        alpha,
                    )
                    .premultiply();
            }
        }

        let left = bitmap.left;
        let top = bitmap.top;

        canvas.draw_pixmap(
            left - 1,
            top - 1,
            pixmap.as_ref(),
            &sk::PixmapPaint::default(),
            sk::Transform::identity(),
            state.mask,
        );
    } else {
        let cw = canvas.width() as i32;
        let ch = canvas.height() as i32;
        let mw = bitmap.width as i32;
        let mh = bitmap.height as i32;

        // Determine the pixel bounding box that we actually need to draw.
        let left = bitmap.left;
        let right = left + mw;
        let top = bitmap.top;
        let bottom = top + mh;

        // Blend the glyph bitmap with the existing pixels on the canvas.
        let pixels = bytemuck::cast_slice_mut::<u8, u32>(canvas.data_mut());
        for x in left.clamp(0, cw)..right.clamp(0, cw) {
            for y in top.clamp(0, ch)..bottom.clamp(0, ch) {
                let ai = ((y - top) * mw + (x - left)) as usize;
                let cov = bitmap.coverage[ai];
                if cov == 0 {
                    continue;
                }

                let color: sk::ColorU8 = sampler.sample((x as _, y as _)).into();
                let color = bytemuck::cast(color.premultiply());
                let pi = (y * cw + x) as usize;
                if cov == 255 {
                    pixels[pi] = color;
                    continue;
                }

                let applied = alpha_mul(color, cov as u32);
                pixels[pi] = blend_src_over(applied, pixels[pi]);
            }
        }
    }

    Some(())
}

/// Render a geometrical shape into the canvas.
fn render_shape(canvas: &mut sk::Pixmap, state: State, shape: &Shape) -> Option<()> {
    let ts = state.transform;
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

    if let Some(fill) = &shape.fill {
        let mut pixmap = None;
        let mut paint: sk::Paint = to_sk_paint(
            fill,
            state,
            shape.geometry.bbox_size(),
            false,
            None,
            &mut pixmap,
            None,
        );

        if matches!(shape.geometry, Geometry::Rect(_)) {
            paint.anti_alias = false;
        }

        let rule = sk::FillRule::default();
        canvas.fill_path(&path, &paint, rule, ts, state.mask);
    }

    if let Some(FixedStroke {
        paint,
        thickness,
        line_cap,
        line_join,
        dash_pattern,
        miter_limit,
    }) = &shape.stroke
    {
        let width = thickness.to_f32();

        // Don't draw zero-pt stroke.
        if width > 0.0 {
            let dash = dash_pattern.as_ref().and_then(|pattern| {
                // tiny-skia only allows dash patterns with an even number of elements,
                // while pdf allows any number.
                let pattern_len = pattern.array.len();
                let len =
                    if pattern_len % 2 == 1 { 2 * pattern_len } else { pattern_len };
                let dash_array =
                    pattern.array.iter().map(|l| l.to_f32()).cycle().take(len).collect();

                sk::StrokeDash::new(dash_array, pattern.phase.to_f32())
            });

            let bbox = shape.geometry.bbox_size();
            let offset_bbox = (!matches!(shape.geometry, Geometry::Line(..)))
                .then(|| offset_bounding_box(bbox, *thickness))
                .unwrap_or(bbox);

            let fill_transform =
                (!matches!(shape.geometry, Geometry::Line(..))).then(|| {
                    sk::Transform::from_translate(
                        -thickness.to_f32(),
                        -thickness.to_f32(),
                    )
                });

            let gradient_map =
                (!matches!(shape.geometry, Geometry::Line(..))).then(|| {
                    (
                        Point::new(
                            -*thickness * state.pixel_per_pt as f64,
                            -*thickness * state.pixel_per_pt as f64,
                        ),
                        Axes::new(
                            Ratio::new(offset_bbox.x / bbox.x),
                            Ratio::new(offset_bbox.y / bbox.y),
                        ),
                    )
                });

            let mut pixmap = None;
            let paint = to_sk_paint(
                paint,
                state,
                offset_bbox,
                false,
                fill_transform,
                &mut pixmap,
                gradient_map,
            );
            let stroke = sk::Stroke {
                width,
                line_cap: line_cap.into(),
                line_join: line_join.into(),
                dash,
                miter_limit: miter_limit.get() as f32,
            };
            canvas.stroke_path(&path, &paint, &stroke, ts, state.mask);
        }
    }

    Some(())
}

/// Convert a Typst path into a tiny-skia path.
fn convert_path(path: &geom::Path) -> Option<sk::Path> {
    let mut builder = sk::PathBuilder::new();
    for elem in &path.0 {
        match elem {
            PathItem::MoveTo(p) => {
                builder.move_to(p.x.to_f32(), p.y.to_f32());
            }
            PathItem::LineTo(p) => {
                builder.line_to(p.x.to_f32(), p.y.to_f32());
            }
            PathItem::CubicTo(p1, p2, p3) => {
                builder.cubic_to(
                    p1.x.to_f32(),
                    p1.y.to_f32(),
                    p2.x.to_f32(),
                    p2.y.to_f32(),
                    p3.x.to_f32(),
                    p3.y.to_f32(),
                );
            }
            PathItem::ClosePath => {
                builder.close();
            }
        };
    }
    builder.finish()
}

/// Render a raster or SVG image into the canvas.
fn render_image(
    canvas: &mut sk::Pixmap,
    state: State,
    image: &Image,
    size: Size,
) -> Option<()> {
    let ts = state.transform;
    let view_width = size.x.to_f32();
    let view_height = size.y.to_f32();

    // For better-looking output, resize `image` to its final size before
    // painting it to `canvas`. For the math, see:
    // https://github.com/typst/typst/issues/1404#issuecomment-1598374652
    let theta = f32::atan2(-ts.kx, ts.sx);

    // To avoid division by 0, choose the one of { sin, cos } that is
    // further from 0.
    let prefer_sin = theta.sin().abs() > std::f32::consts::FRAC_1_SQRT_2;
    let scale_x =
        f32::abs(if prefer_sin { ts.kx / theta.sin() } else { ts.sx / theta.cos() });

    let aspect = (image.width() as f32) / (image.height() as f32);
    let w = (scale_x * view_width.max(aspect * view_height)).ceil() as u32;
    let h = ((w as f32) / aspect).ceil() as u32;

    let pixmap = scaled_texture(image, w, h)?;
    let paint_scale_x = view_width / pixmap.width() as f32;
    let paint_scale_y = view_height / pixmap.height() as f32;

    let paint = sk::Paint {
        shader: sk::Pattern::new(
            (*pixmap).as_ref(),
            sk::SpreadMode::Pad,
            sk::FilterQuality::Nearest,
            1.0,
            sk::Transform::from_scale(paint_scale_x, paint_scale_y),
        ),
        ..Default::default()
    };

    let rect = sk::Rect::from_xywh(0.0, 0.0, view_width, view_height)?;
    canvas.fill_rect(rect, &paint, ts, state.mask);

    Some(())
}

/// Prepare a texture for an image at a scaled size.
#[comemo::memoize]
fn scaled_texture(image: &Image, w: u32, h: u32) -> Option<Arc<sk::Pixmap>> {
    let mut pixmap = sk::Pixmap::new(w, h)?;
    match image.kind() {
        ImageKind::Raster(raster) => {
            let downscale = w < image.width();
            let filter =
                if downscale { FilterType::Lanczos3 } else { FilterType::CatmullRom };
            let buf = raster.dynamic().resize(w, h, filter);
            for ((_, _, src), dest) in buf.pixels().zip(pixmap.pixels_mut()) {
                let Rgba([r, g, b, a]) = src;
                *dest = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
            }
        }
        // Safety: We do not keep any references to tree nodes beyond the scope
        // of `with`.
        ImageKind::Svg(svg) => unsafe {
            svg.with(|tree| {
                let tree = resvg::Tree::from_usvg(tree);
                let ts = tiny_skia::Transform::from_scale(
                    w as f32 / tree.size.width(),
                    h as f32 / tree.size.height(),
                );
                tree.render(ts, &mut pixmap.as_mut())
            });
        },
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

impl From<sk::Transform> for Transform {
    fn from(value: sk::Transform) -> Self {
        let sk::Transform { sx, ky, kx, sy, tx, ty } = value;
        Self {
            sx: Ratio::new(sx as _),
            ky: Ratio::new(ky as _),
            kx: Ratio::new(kx as _),
            sy: Ratio::new(sy as _),
            tx: Abs::raw(tx as _),
            ty: Abs::raw(ty as _),
        }
    }
}

/// Trait for sampling of a paint, used as a generic
/// abstraction over solid colors and gradients.
trait PaintSampler: Copy {
    /// Sample the color at the `pos` in the pixmap.
    fn sample(self, pos: (u32, u32)) -> Color;
}

impl PaintSampler for Color {
    fn sample(self, _: (u32, u32)) -> Color {
        self
    }
}

/// State used when sampling colors for text.
///
/// It caches the inverse transform to the parent, so that we can
/// reuse it instead of recomputing it for each pixel.
#[derive(Clone, Copy)]
struct GradientSampler<'a> {
    gradient: &'a Gradient,
    container_size: Size,
    transform_to_parent: sk::Transform,
}

impl<'a> GradientSampler<'a> {
    fn new(
        gradient: &'a Gradient,
        state: &State,
        item_size: Size,
        on_text: bool,
    ) -> Self {
        let relative = gradient.unwrap_relative(on_text);
        let container_size = match relative {
            Relative::Self_ => item_size,
            Relative::Parent => state.size,
        };

        let fill_transform = match relative {
            Relative::Self_ => sk::Transform::identity(),
            Relative::Parent => state.container_transform.invert().unwrap(),
        };

        Self {
            gradient,
            container_size,
            transform_to_parent: fill_transform,
        }
    }
}

impl PaintSampler for GradientSampler<'_> {
    /// Samples a single point in a glyph.
    fn sample(self, (x, y): (u32, u32)) -> Color {
        // Compute the point in the gradient's coordinate space.
        let mut point = sk::Point { x: x as f32, y: y as f32 };
        self.transform_to_parent.map_point(&mut point);

        // Sample the gradient
        self.gradient.sample_at(
            (point.x, point.y),
            (self.container_size.x.to_f32(), self.container_size.y.to_f32()),
        )
    }
}

/// Transforms a [`Paint`] into a [`sk::Paint`].
/// Applying the necessary transform, if the paint is a gradient.
///
/// `gradient_map` is used to scale and move the gradient being sampled,
/// this is used to line up the stroke and the fill of a shape.
fn to_sk_paint<'a>(
    paint: &Paint,
    state: State,
    item_size: Size,
    on_text: bool,
    fill_transform: Option<sk::Transform>,
    pixmap: &'a mut Option<Arc<sk::Pixmap>>,
    gradient_map: Option<(Point, Axes<Ratio>)>,
) -> sk::Paint<'a> {
    /// Actual sampling of the gradient, cached for performance.
    #[comemo::memoize]
    fn cached(
        gradient: &Gradient,
        width: u32,
        height: u32,
        gradient_map: Option<(Point, Axes<Ratio>)>,
    ) -> Arc<sk::Pixmap> {
        let (offset, scale) =
            gradient_map.unwrap_or_else(|| (Point::zero(), Axes::splat(Ratio::one())));
        let mut pixmap = sk::Pixmap::new(width.max(1), height.max(1)).unwrap();
        for x in 0..width {
            for y in 0..height {
                let color: sk::Color = gradient
                    .sample_at(
                        (
                            (x as f32 + offset.x.to_f32()) * scale.x.get() as f32,
                            (y as f32 + offset.y.to_f32()) * scale.y.get() as f32,
                        ),
                        (width as f32, height as f32),
                    )
                    .into();

                pixmap.pixels_mut()[(y * width + x) as usize] =
                    color.premultiply().to_color_u8();
            }
        }

        Arc::new(pixmap)
    }

    let mut sk_paint: sk::Paint<'_> = sk::Paint::default();
    match paint {
        Paint::Solid(color) => {
            sk_paint.set_color((*color).into());
            sk_paint.anti_alias = true;
        }
        Paint::Gradient(gradient) => {
            let relative = gradient.unwrap_relative(on_text);
            let container_size = match relative {
                Relative::Self_ => item_size,
                Relative::Parent => state.size,
            };

            let fill_transform = match relative {
                Relative::Self_ => fill_transform.unwrap_or_default(),
                Relative::Parent => state
                    .container_transform
                    .post_concat(state.transform.invert().unwrap()),
            };
            let width = (container_size.x.to_f32() * state.pixel_per_pt).ceil() as u32;
            let height = (container_size.y.to_f32() * state.pixel_per_pt).ceil() as u32;

            *pixmap = Some(cached(
                gradient,
                width.max(state.pixel_per_pt.ceil() as u32),
                height.max(state.pixel_per_pt.ceil() as u32),
                gradient_map,
            ));

            // We can use FilterQuality::Nearest here because we're
            // rendering to a pixmap that is already at native resolution.
            sk_paint.shader = sk::Pattern::new(
                pixmap.as_ref().unwrap().as_ref().as_ref(),
                sk::SpreadMode::Pad,
                sk::FilterQuality::Nearest,
                1.0,
                fill_transform
                    .pre_scale(1.0 / state.pixel_per_pt, 1.0 / state.pixel_per_pt),
            );

            sk_paint.anti_alias = gradient.anti_alias();
        }
    }

    sk_paint
}

impl From<Color> for sk::Color {
    fn from(color: Color) -> Self {
        let [r, g, b, a] = color.to_rgba().to_vec4_u8();
        sk::Color::from_rgba8(r, g, b, a)
    }
}

impl From<&LineCap> for sk::LineCap {
    fn from(line_cap: &LineCap) -> Self {
        match line_cap {
            LineCap::Butt => sk::LineCap::Butt,
            LineCap::Round => sk::LineCap::Round,
            LineCap::Square => sk::LineCap::Square,
        }
    }
}

impl From<&LineJoin> for sk::LineJoin {
    fn from(line_join: &LineJoin) -> Self {
        match line_join {
            LineJoin::Miter => sk::LineJoin::Miter,
            LineJoin::Round => sk::LineJoin::Round,
            LineJoin::Bevel => sk::LineJoin::Bevel,
        }
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

impl From<Color> for sk::ColorU8 {
    fn from(value: Color) -> Self {
        let [r, g, b, _] = value.to_rgba().to_vec4_u8();
        sk::ColorU8::from_rgba(r, g, b, 255)
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

fn offset_bounding_box(bbox: Size, stroke_width: Abs) -> Size {
    Size::new(bbox.x + stroke_width * 2.0, bbox.y + stroke_width * 2.0)
}

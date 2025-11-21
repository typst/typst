use std::sync::Arc;

use pixglyph::Bitmap;
use tiny_skia as sk;
use ttf_parser::{GlyphId, OutlineBuilder};
use typst_library::layout::{Abs, Axes, Point, Size};
use typst_library::text::color::{glyph_frame, should_outline};
use typst_library::text::{Font, TextItem};
use typst_library::visualize::{FixedStroke, Paint};

use crate::paint::{self, GradientSampler, PaintSampler, TilingSampler};
use crate::{AbsExt, State, shape};

/// Render a text run into the canvas.
pub fn render_text(canvas: &mut sk::Pixmap, state: State, text: &TextItem) {
    let mut x = Abs::zero();
    let mut y = Abs::zero();
    for glyph in &text.glyphs {
        let id = GlyphId(glyph.id);
        let x_offset = x + glyph.x_offset.at(text.size);
        let y_offset = y + glyph.y_offset.at(text.size);

        if should_outline(&text.font, glyph) {
            let state = state.pre_translate(Point::new(x_offset, -y_offset));
            render_outline_glyph(canvas, state, text, id);
        } else {
            let upem = text.font.units_per_em();
            let text_scale = text.size / upem;
            let state = state
                .pre_translate(Point::new(x_offset, -y_offset - text.size))
                .pre_scale(Axes::new(text_scale, text_scale));

            let (glyph_frame, _) = glyph_frame(&text.font, glyph.id);
            crate::render_frame(canvas, state, &glyph_frame);
        }

        x += glyph.x_advance.at(text.size);
        y += glyph.y_advance.at(text.size);
    }
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
    if ppem > 100.0
        || ts.kx != 0.0
        || ts.ky != 0.0
        || ts.sx != ts.sy
        || text.stroke.is_some()
        || text.size < Abs::zero()
    {
        let path = {
            let mut builder = WrappedPathBuilder(sk::PathBuilder::new());
            text.font.ttf().outline_glyph(id, &mut builder)?;
            builder.0.finish()?
        };

        let scale = text.size.to_f32() / text.font.units_per_em() as f32;

        let mut pixmap = None;

        let rule = sk::FillRule::default();

        // Flip vertically because font design coordinate
        // system is Y-up.
        let ts = ts.pre_scale(scale, -scale);
        let state_ts = state.pre_concat(sk::Transform::from_scale(scale, -scale));
        let paint = paint::to_sk_paint(
            &text.fill,
            state_ts,
            Size::zero(),
            true,
            None,
            &mut pixmap,
            None,
        );
        canvas.fill_path(&path, &paint, rule, ts, state.mask);

        if let Some(FixedStroke { paint, thickness, cap, join, dash, miter_limit }) =
            &text.stroke
            && thickness.to_f32() > 0.0
        {
            let dash = dash.as_ref().and_then(shape::to_sk_dash_pattern);

            let paint = paint::to_sk_paint(
                paint,
                state_ts,
                Size::zero(),
                true,
                None,
                &mut pixmap,
                None,
            );
            let stroke = sk::Stroke {
                width: thickness.to_f32() / scale, // When we scale the path, we need to scale the stroke width, too.
                line_cap: shape::to_sk_line_cap(*cap),
                line_join: shape::to_sk_line_join(*join),
                dash,
                miter_limit: miter_limit.get() as f32,
            };

            canvas.stroke_path(&path, &paint, &stroke, ts, state.mask);
        }
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
            write_bitmap(
                canvas,
                &bitmap,
                &state,
                paint::to_sk_color_u8(*color).premultiply(),
            )?;
        }
        Paint::Tiling(tiling) => {
            let pixmap = paint::render_tiling_frame(&state, tiling);
            let sampler = TilingSampler::new(tiling, &pixmap, &state, true);
            write_bitmap(canvas, &bitmap, &state, sampler)?;
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
        let cw = canvas.width() as i32;
        let ch = canvas.height() as i32;
        let mw = bitmap.width;
        let mh = bitmap.height;

        let left = bitmap.left;
        let top = bitmap.top;

        // Pad the pixmap with 1 pixel in each dimension so that we do
        // not get any problem with floating point errors along their border
        let mut pixmap = sk::Pixmap::new(mw + 2, mh + 2)?;
        let pixels = bytemuck::cast_slice_mut::<u8, u32>(pixmap.data_mut());
        for x in 0..mw {
            for y in 0..mh {
                let alpha = bitmap.coverage[(y * mw + x) as usize];

                // To sample at the correct position, we need to convert each
                // pixel's position in the bitmap (x and y) to its final
                // expected position in the canvas. Due to padding, this
                // pixel's position in the pixmap will be (x + 1, y + 1).
                // Then, when drawing the pixmap to the canvas, we place its
                // top-left corner at position (left - 1, top - 1). Therefore,
                // the final position of this pixel in the canvas is given by
                // (left - 1 + x + 1, top - 1 + y + 1) = (left + x, top + y).
                let sample_pos = (
                    (left + x as i32).clamp(0, cw) as u32,
                    (top + y as i32).clamp(0, ch) as u32,
                );
                let color = sampler.sample(sample_pos);
                let color = bytemuck::cast(color);

                let applied = alpha_mul(color, alpha as u32);
                pixels[((y + 1) * (mw + 2) + (x + 1)) as usize] = applied;
            }
        }

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

                let color = sampler.sample((x as _, y as _));
                let color = bytemuck::cast(color);
                let pi = (y * cw + x) as usize;
                // Fast path if color is opaque.
                if cov == u8::MAX && color & 0xFF == 0xFF {
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

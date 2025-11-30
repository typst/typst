use std::sync::Arc;

use tiny_skia as sk;
use typst_library::layout::{Axes, Point, Ratio, Size};
use typst_library::visualize::{Color, Gradient, Paint, RelativeTo, Tiling};

use crate::{AbsExt, State};

/// Trait for sampling of a paint, used as a generic
/// abstraction over solid colors and gradients.
pub trait PaintSampler: Copy {
    /// Sample the color at the `pos` in the pixmap.
    fn sample(self, pos: (u32, u32)) -> sk::PremultipliedColorU8;
}

impl PaintSampler for sk::PremultipliedColorU8 {
    fn sample(self, _: (u32, u32)) -> sk::PremultipliedColorU8 {
        self
    }
}

/// State used when sampling colors for text.
///
/// It caches the inverse transform to the parent, so that we can
/// reuse it instead of recomputing it for each pixel.
#[derive(Copy, Clone)]
pub struct GradientSampler<'a> {
    gradient: &'a Gradient,
    container_size: Size,
    transform_to_parent: sk::Transform,
}

impl<'a> GradientSampler<'a> {
    pub fn new(
        gradient: &'a Gradient,
        state: &State,
        item_size: Size,
        on_text: bool,
    ) -> Self {
        let relative = gradient.unwrap_relative(on_text);
        let container_size = match relative {
            RelativeTo::Self_ => item_size,
            RelativeTo::Parent => state.size,
        };

        let fill_transform = match relative {
            RelativeTo::Self_ => sk::Transform::identity(),
            RelativeTo::Parent => state.container_transform.invert().unwrap(),
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
    fn sample(self, (x, y): (u32, u32)) -> sk::PremultipliedColorU8 {
        // Compute the point in the gradient's coordinate space.
        let mut point = sk::Point { x: x as f32, y: y as f32 };
        self.transform_to_parent.map_point(&mut point);

        // Sample the gradient
        to_sk_color_u8(self.gradient.sample_at(
            (point.x, point.y),
            (self.container_size.x.to_f32(), self.container_size.y.to_f32()),
        ))
        .premultiply()
    }
}

/// State used when sampling tilings for text.
///
/// It caches the inverse transform to the parent, so that we can
/// reuse it instead of recomputing it for each pixel.
#[derive(Copy, Clone)]
pub struct TilingSampler<'a> {
    size: Size,
    transform_to_parent: sk::Transform,
    pixmap: &'a sk::Pixmap,
    pixel_per_pt: f32,
}

impl<'a> TilingSampler<'a> {
    pub fn new(
        tilings: &'a Tiling,
        pixmap: &'a sk::Pixmap,
        state: &State,
        on_text: bool,
    ) -> Self {
        let relative = tilings.unwrap_relative(on_text);
        let fill_transform = match relative {
            RelativeTo::Self_ => sk::Transform::identity(),
            RelativeTo::Parent => state.container_transform.invert().unwrap(),
        };

        Self {
            pixmap,
            size: (tilings.size() + tilings.spacing()) * state.pixel_per_pt as f64,
            transform_to_parent: fill_transform,
            pixel_per_pt: state.pixel_per_pt,
        }
    }
}

impl PaintSampler for TilingSampler<'_> {
    /// Samples a single point in a glyph.
    fn sample(self, (x, y): (u32, u32)) -> sk::PremultipliedColorU8 {
        // Compute the point in the tilings's coordinate space.
        let mut point = sk::Point { x: x as f32, y: y as f32 };
        self.transform_to_parent.map_point(&mut point);

        let x =
            (point.x * self.pixel_per_pt).rem_euclid(self.size.x.to_f32()).floor() as u32;
        let y =
            (point.y * self.pixel_per_pt).rem_euclid(self.size.y.to_f32()).floor() as u32;

        // Sample the tilings
        self.pixmap.pixel(x, y).unwrap()
    }
}

/// Transforms a [`Paint`] into a [`sk::Paint`].
/// Applying the necessary transform, if the paint is a gradient.
///
/// `gradient_map` is used to scale and move the gradient being sampled,
/// this is used to line up the stroke and the fill of a shape.
pub fn to_sk_paint<'a>(
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
                let color = gradient.sample_at(
                    (
                        (x as f32 + offset.x.to_f32()) * scale.x.get() as f32,
                        (y as f32 + offset.y.to_f32()) * scale.y.get() as f32,
                    ),
                    (width as f32, height as f32),
                );

                pixmap.pixels_mut()[(y * width + x) as usize] =
                    to_sk_color(color).premultiply().to_color_u8();
            }
        }

        Arc::new(pixmap)
    }

    let mut sk_paint: sk::Paint<'_> = sk::Paint::default();
    match paint {
        Paint::Solid(color) => {
            sk_paint.set_color(to_sk_color(*color));
            sk_paint.anti_alias = true;
        }
        Paint::Gradient(gradient) => {
            let relative = gradient.unwrap_relative(on_text);
            let container_size = match relative {
                RelativeTo::Self_ => item_size,
                RelativeTo::Parent => state.size,
            };

            let fill_transform = match relative {
                RelativeTo::Self_ => fill_transform.unwrap_or_default(),
                RelativeTo::Parent => state
                    .container_transform
                    .post_concat(state.transform.invert().unwrap()),
            };

            let gradient_map = match relative {
                RelativeTo::Self_ => gradient_map,
                RelativeTo::Parent => None,
            };

            let width =
                (container_size.x.to_f32().abs() * state.pixel_per_pt).ceil() as u32;
            let height =
                (container_size.y.to_f32().abs() * state.pixel_per_pt).ceil() as u32;

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
                fill_transform.pre_scale(
                    container_size.x.signum() as f32 / state.pixel_per_pt,
                    container_size.y.signum() as f32 / state.pixel_per_pt,
                ),
            );

            sk_paint.anti_alias = gradient.anti_alias();
        }
        Paint::Tiling(tilings) => {
            let relative = tilings.unwrap_relative(on_text);

            let fill_transform = match relative {
                RelativeTo::Self_ => fill_transform.unwrap_or_default(),
                RelativeTo::Parent => state
                    .container_transform
                    .post_concat(state.transform.invert().unwrap()),
            };

            let canvas = render_tiling_frame(&state, tilings);
            *pixmap = Some(Arc::new(canvas));

            let offset = match relative {
                RelativeTo::Self_ => {
                    let base_offset = gradient_map
                        .map(|(offset, _)| -offset)
                        .unwrap_or_default();
                    Point::new(
                        base_offset.x + tilings.dx(),
                        base_offset.y + tilings.dy(),
                    )
                }
                RelativeTo::Parent => {
                    Point::new(tilings.dx(), tilings.dy())
                }
            };

            // Create the shader
            sk_paint.shader = sk::Pattern::new(
                pixmap.as_ref().unwrap().as_ref().as_ref(),
                sk::SpreadMode::Repeat,
                sk::FilterQuality::Nearest,
                1.0,
                fill_transform
                    .pre_translate(offset.x.to_f32(), offset.y.to_f32())
                    .pre_scale(1.0 / state.pixel_per_pt, 1.0 / state.pixel_per_pt),
            );
        }
    }

    sk_paint
}

pub fn to_sk_color(color: Color) -> sk::Color {
    let (r, g, b, a) = color.to_rgb().into_components();
    sk::Color::from_rgba(r, g, b, a)
        .expect("components must always be in the range [0..=1]")
}

pub fn to_sk_color_u8(color: Color) -> sk::ColorU8 {
    let (r, g, b, a) = color.to_rgb().into_format::<u8, u8>().into_components();
    sk::ColorU8::from_rgba(r, g, b, a)
}

pub fn render_tiling_frame(state: &State, tilings: &Tiling) -> sk::Pixmap {
    let size = tilings.size() + tilings.spacing();
    let mut canvas = sk::Pixmap::new(
        (size.x.to_f32() * state.pixel_per_pt).round() as u32,
        (size.y.to_f32() * state.pixel_per_pt).round() as u32,
    )
    .unwrap();

    // Render the tilings into a new canvas.
    let ts = sk::Transform::from_scale(state.pixel_per_pt, state.pixel_per_pt);
    let temp_state = State::new(tilings.size(), ts, state.pixel_per_pt);
    crate::render_frame(&mut canvas, temp_state, tilings.frame());
    canvas
}

//! Rendering of Typst documents into raster images.

mod image;
mod paint;
mod shape;
mod text;

use tiny_skia as sk;
use typst_library::layout::{
    Abs, Axes, Frame, FrameItem, FrameKind, GroupItem, Page, PagedDocument, Point, Size,
    Transform,
};
use typst_library::visualize::{Color, Geometry, Paint};

/// Export a page into a raster image.
///
/// This renders the page at the given number of pixels per point and returns
/// the resulting `tiny-skia` pixel buffer.
#[typst_macros::time(name = "render")]
pub fn render(page: &Page, pixel_per_pt: f32) -> sk::Pixmap {
    let size = page.frame.size();
    let pxw = (pixel_per_pt * size.x.to_f32()).round().max(1.0) as u32;
    let pxh = (pixel_per_pt * size.y.to_f32()).round().max(1.0) as u32;

    let ts = sk::Transform::from_scale(pixel_per_pt, pixel_per_pt);
    let state = State::new(size, ts, pixel_per_pt);

    let mut canvas = sk::Pixmap::new(pxw, pxh).unwrap();

    if let Some(fill) = page.fill_or_white() {
        if let Paint::Solid(color) = fill {
            canvas.fill(paint::to_sk_color(color));
        } else {
            let rect = Geometry::Rect(page.frame.size()).filled(fill);
            shape::render_shape(&mut canvas, state, &rect);
        }
    }

    render_frame(&mut canvas, state, &page.frame);

    canvas
}

/// Export a document with potentially multiple pages into a single raster image.
pub fn render_merged(
    document: &PagedDocument,
    pixel_per_pt: f32,
    gap: Abs,
    fill: Option<Color>,
) -> sk::Pixmap {
    let pixmaps: Vec<_> =
        document.pages.iter().map(|page| render(page, pixel_per_pt)).collect();

    let gap = (pixel_per_pt * gap.to_f32()).round() as u32;
    let pxw = pixmaps.iter().map(sk::Pixmap::width).max().unwrap_or_default();
    let pxh = pixmaps.iter().map(|pixmap| pixmap.height()).sum::<u32>()
        + gap * pixmaps.len().saturating_sub(1) as u32;

    let mut canvas = sk::Pixmap::new(pxw, pxh).unwrap();
    if let Some(fill) = fill {
        canvas.fill(paint::to_sk_color(fill));
    }

    let mut y = 0;
    for pixmap in pixmaps {
        canvas.draw_pixmap(
            0,
            y as i32,
            pixmap.as_ref(),
            &sk::PixmapPaint::default(),
            sk::Transform::identity(),
            None,
        );

        y += pixmap.height() + gap;
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

impl State<'_> {
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

    fn pre_scale(self, scale: Axes<Abs>) -> Self {
        Self {
            transform: self.transform.pre_scale(scale.x.to_f32(), scale.y.to_f32()),
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
                text::render_text(canvas, state.pre_translate(*pos), text);
            }
            FrameItem::Shape(shape, _) => {
                shape::render_shape(canvas, state.pre_translate(*pos), shape);
            }
            FrameItem::Image(image, size, _) => {
                image::render_image(canvas, state.pre_translate(*pos), image, *size);
            }
            FrameItem::Link(_, _) => {}
            FrameItem::Tag(_) => {}
        }
    }
}

/// Render a group frame with optional transform and clipping into the canvas.
fn render_group(canvas: &mut sk::Pixmap, state: State, pos: Point, group: &GroupItem) {
    let sk_transform = to_sk_transform(&group.transform);
    let state = match group.frame.kind() {
        FrameKind::Soft => state.pre_translate(pos).pre_concat(sk_transform),
        FrameKind::Hard => state
            .pre_translate(pos)
            .pre_concat(sk_transform)
            .pre_concat_container(
                state
                    .transform
                    .post_concat(state.container_transform.invert().unwrap()),
            )
            .pre_concat_container(to_sk_transform(&Transform::translate(pos.x, pos.y)))
            .pre_concat_container(sk_transform)
            .with_size(group.frame.size()),
    };

    let mut mask = state.mask;
    let storage;
    if let Some(clip_path) = group.clip_path.as_ref() {
        if let Some(path) = shape::convert_path(clip_path)
            .and_then(|path| path.transform(state.transform))
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

fn to_sk_transform(transform: &Transform) -> sk::Transform {
    let Transform { sx, ky, kx, sy, tx, ty } = *transform;
    sk::Transform::from_row(
        sx.get() as _,
        ky.get() as _,
        kx.get() as _,
        sy.get() as _,
        tx.to_f32(),
        ty.to_f32(),
    )
}

/// Additional methods for [`Abs`].
trait AbsExt {
    /// Convert to a number of points as f32.
    fn to_f32(self) -> f32;
}

impl AbsExt for Abs {
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}

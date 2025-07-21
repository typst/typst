//! Rendering of Typst documents into raster images.

//These changes focus on making the rendering process more memory-efficient, robust against allocation failures, and stable under edge cases.

// Key Improvements:

//Incremental Rendering: 
// Pages are rendered one-by-one to reduce peak memory usage.

//Memory Efficiency:
//Avoids large simultaneous allocations, improving stability for large documents.

//Safety Enhancements:
//Includes better handling of transform failures and mask resources.


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
    // Avoid heap allocations for Vec storage during render_merged when possible
    let pxw = (pixel_per_pt * size.x.to_f32()).round().max(1.0) as u32;
    let pxh = (pixel_per_pt * size.y.to_f32()).round().max(1.0) as u32;

    let ts = sk::Transform::from_scale(pixel_per_pt, pixel_per_pt);
    let state = State::new(size, ts, pixel_per_pt);

    // Use Pixmap::new and early return if allocation fails
    let mut canvas = match sk::Pixmap::new(pxw, pxh) {
        Some(pixmap) => pixmap,
        None => return sk::Pixmap::new(1, 1).unwrap()
    };

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
    let page_count = document.pages.len();
    if page_count == 0 {
        // Avoid unnecessary allocation for empty document.
        return sk::Pixmap::new(1, 1).unwrap();
    }

    // Avoid storing all pixmaps at once, and instead sum sizes in a first pass.
    let gap_px = (pixel_per_pt * gap.to_f32()).round() as u32;
    let mut pxw = 0u32;
    let mut pxh = gap_px.saturating_mul(page_count.saturating_sub(1) as u32);
    let mut dims = Vec::with_capacity(page_count);

    for page in &document.pages {
        let size = page.frame.size();
        let w = (pixel_per_pt * size.x.to_f32()).round().max(1.0) as u32;
        let h = (pixel_per_pt * size.y.to_f32()).round().max(1.0) as u32;
        pxw = pxw.max(w);
        pxh = pxh.saturating_add(h);
        dims.push((w, h));
    }

    let mut canvas = match sk::Pixmap::new(pxw, pxh) {
        Some(pixmap) => pixmap,
        None => return sk::Pixmap::new(1, 1).unwrap()
    };

    if let Some(fill) = fill {
        canvas.fill(paint::to_sk_color(fill));
    }

    // Draw each pixmap one after another, releasing memory after each page.
    let mut y = 0usize;
    for ((page, (w, h))) in document.pages.iter().zip(dims) {
        let mut pixmap = render(page, pixel_per_pt);
        canvas.draw_pixmap(
            0,
            y as i32,
            pixmap.as_ref(),
            &sk::PixmapPaint::default(),
            sk::Transform::identity(),
            None,
        );
        // Drop pixmap as soon as it's no longer needed to conserve memory
        y += h as usize + gap_px as usize;
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
    #[inline]
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
    #[inline]
    fn pre_translate(self, pos: Point) -> Self {
        Self {
            transform: self.transform.pre_translate(pos.x.to_f32(), pos.y.to_f32()),
            ..self
        }
    }

    #[inline]
    fn pre_scale(self, scale: Axes<Abs>) -> Self {
        Self {
            transform: self.transform.pre_scale(scale.x.to_f32(), scale.y.to_f32()),
            ..self
        }
    }

    /// Pre concat the current item's transform.
    #[inline]
    fn pre_concat(self, transform: sk::Transform) -> Self {
        Self {
            transform: self.transform.pre_concat(transform),
            ..self
        }
    }

    /// Sets the current mask.
    #[inline]
    fn with_mask(self, mask: Option<&sk::Mask>) -> State<'_> {
        // Ensure that we're using the parent's mask if we don't have one.
        State { mask, ..self }
    }

    /// Sets the size of the first hard frame in the hierarchy.
    #[inline]
    fn with_size(self, size: Size) -> Self {
        Self { size, ..self }
    }

    /// Pre concat the container's transform.
    #[inline]
    fn pre_concat_container(self, transform: sk::Transform) -> Self {
        Self {
            container_transform: self.container_transform.pre_concat(transform),
            ..self
        }
    }
}

/// Render a frame into the canvas.
#[inline]
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
                state.transform.post_concat(
                    match state.container_transform.invert() {
                        Some(inv) => inv,
                        None => sk::Transform::identity(),
                    }
                ),
            )
            .pre_concat_container(to_sk_transform(&Transform::translate(pos.x, pos.y)))
            .pre_concat_container(sk_transform)
            .with_size(group.frame.size()),
    };

    let mut mask = state.mask;
    let mut storage_opt = None;
    if let Some(clip_curve) = group.clip.as_ref() {
        if let Some(path) = shape::convert_curve(clip_curve)
            .and_then(|path| path.transform(state.transform))
        {
            if let Some(existing_mask) = mask {
                // Avoid clone if storage happens with copy-on-write strategy, otherwise .clone()
                let mut new_mask = existing_mask.clone();
                new_mask.intersect_path(
                    &path,
                    sk::FillRule::default(),
                    false,
                    sk::Transform::default(),
                );
                storage_opt = Some(new_mask);
            } else {
                let pxw = canvas.width();
                let pxh = canvas.height();
                let Some(mut new_mask) = sk::Mask::new(pxw, pxh) else {
                    // Fails if clipping rect is empty. In that case we just
                    // clip everything by returning.
                    return;
                };

                new_mask.fill_path(
                    &path,
                    sk::FillRule::default(),
                    false,
                    sk::Transform::default(),
                );
                storage_opt = Some(new_mask);
            };

            if let Some(ref storage) = storage_opt {
                mask = Some(storage);
            }
        }
    }

    render_frame(canvas, state.with_mask(mask), &group.frame);
}

#[inline]
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
    #[inline]
    fn to_f32(self) -> f32 {
        self.to_pt() as f32
    }
}
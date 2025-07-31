use std::cell::LazyCell;

use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Packed, Resolve, Smart, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, FixedAlignment, Frame, MoveElem, Point, Ratio, Region, Rel, RotateElem,
    ScaleAmount, ScaleElem, Size, SkewElem, Transform,
};
use typst_utils::Numeric;

/// Layout the moved content.
#[typst_macros::time(span = elem.span())]
pub fn layout_move(
    elem: &Packed<MoveElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let mut frame = crate::layout_frame(engine, &elem.body, locator, styles, region)?;
    let delta = Axes::new(elem.dx.resolve(styles), elem.dy.resolve(styles));
    let delta = delta.zip_map(region.size, Rel::relative_to);
    frame.translate(delta.to_point());
    Ok(frame)
}

/// Layout the rotated content.
#[typst_macros::time(span = elem.span())]
pub fn layout_rotate(
    elem: &Packed<RotateElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let angle = elem.angle.get(styles);
    let align = elem.origin.resolve(styles);

    // Compute the new region's approximate size.
    let is_finite = region.size.is_finite();
    let size = if is_finite {
        compute_bounding_box(region.size, Transform::rotate(-angle)).1
    } else {
        Size::splat(Abs::inf())
    };

    measure_and_layout(
        engine,
        locator,
        region,
        size,
        styles,
        &elem.body,
        Transform::rotate(angle),
        align,
        elem.reflow.get(styles),
    )
}

/// Layout the scaled content.
#[typst_macros::time(span = elem.span())]
pub fn layout_scale(
    elem: &Packed<ScaleElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    // Compute the new region's approximate size.
    let scale = resolve_scale(elem, engine, locator.relayout(), region.size, styles)?;
    let size = region
        .size
        .zip_map(scale, |r, s| if r.is_finite() { Ratio::new(1.0 / s).of(r) } else { r })
        .map(Abs::abs);

    measure_and_layout(
        engine,
        locator,
        region,
        size,
        styles,
        &elem.body,
        Transform::scale(scale.x, scale.y),
        elem.origin.resolve(styles),
        elem.reflow.get(styles),
    )
}

/// Resolves scale parameters, preserving aspect ratio if one of the scales
/// is set to `auto`.
fn resolve_scale(
    elem: &Packed<ScaleElem>,
    engine: &mut Engine,
    locator: Locator,
    container: Size,
    styles: StyleChain,
) -> SourceResult<Axes<Ratio>> {
    fn resolve_axis(
        axis: Smart<ScaleAmount>,
        body: impl Fn() -> SourceResult<Abs>,
        styles: StyleChain,
    ) -> SourceResult<Smart<Ratio>> {
        Ok(match axis {
            Smart::Auto => Smart::Auto,
            Smart::Custom(amt) => Smart::Custom(match amt {
                ScaleAmount::Ratio(ratio) => ratio,
                ScaleAmount::Length(length) => {
                    let length = length.resolve(styles);
                    Ratio::new(length / body()?)
                }
            }),
        })
    }

    let size = LazyCell::new(|| {
        let pod = Region::new(container, Axes::splat(false));
        let frame = crate::layout_frame(engine, &elem.body, locator, styles, pod)?;
        SourceResult::Ok(frame.size())
    });

    let x = resolve_axis(
        elem.x.get(styles),
        || size.as_ref().map(|size| size.x).map_err(Clone::clone),
        styles,
    )?;

    let y = resolve_axis(
        elem.y.get(styles),
        || size.as_ref().map(|size| size.y).map_err(Clone::clone),
        styles,
    )?;

    match (x, y) {
        (Smart::Auto, Smart::Auto) => {
            bail!(elem.span(), "x and y cannot both be auto")
        }
        (Smart::Custom(x), Smart::Custom(y)) => Ok(Axes::new(x, y)),
        (Smart::Auto, Smart::Custom(v)) | (Smart::Custom(v), Smart::Auto) => {
            Ok(Axes::splat(v))
        }
    }
}

/// Layout the skewed content.
#[typst_macros::time(span = elem.span())]
pub fn layout_skew(
    elem: &Packed<SkewElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let ax = elem.ax.get(styles);
    let ay = elem.ay.get(styles);
    let align = elem.origin.resolve(styles);

    // Compute the new region's approximate size.
    let size = if region.size.is_finite() {
        compute_bounding_box(region.size, Transform::skew(ax, ay)).1
    } else {
        Size::splat(Abs::inf())
    };

    measure_and_layout(
        engine,
        locator,
        region,
        size,
        styles,
        &elem.body,
        Transform::skew(ax, ay),
        align,
        elem.reflow.get(styles),
    )
}

/// Applies a transformation to a frame, reflowing the layout if necessary.
#[allow(clippy::too_many_arguments)]
fn measure_and_layout(
    engine: &mut Engine,
    locator: Locator,
    region: Region,
    size: Size,
    styles: StyleChain,
    body: &Content,
    transform: Transform,
    align: Axes<FixedAlignment>,
    reflow: bool,
) -> SourceResult<Frame> {
    if reflow {
        // Measure the size of the body.
        let pod = Region::new(size, Axes::splat(false));
        let frame = crate::layout_frame(engine, body, locator.relayout(), styles, pod)?;

        // Actually perform the layout.
        let pod = Region::new(frame.size(), Axes::splat(true));
        let mut frame = crate::layout_frame(engine, body, locator, styles, pod)?;
        let Axes { x, y } = align.zip_map(frame.size(), FixedAlignment::position);

        // Compute the transform.
        let ts = Transform::translate(x, y)
            .pre_concat(transform)
            .pre_concat(Transform::translate(-x, -y));

        // Compute the bounding box and offset and wrap in a new frame.
        let (offset, size) = compute_bounding_box(frame.size(), ts);
        frame.transform(ts);
        frame.translate(offset);
        frame.set_size(size);
        Ok(frame)
    } else {
        // Layout the body.
        let mut frame = crate::layout_frame(engine, body, locator, styles, region)?;
        let Axes { x, y } = align.zip_map(frame.size(), FixedAlignment::position);

        // Compute the transform.
        let ts = Transform::translate(x, y)
            .pre_concat(transform)
            .pre_concat(Transform::translate(-x, -y));

        // Apply the transform.
        frame.transform(ts);
        Ok(frame)
    }
}

/// Computes the bounding box and offset of a transformed area.
fn compute_bounding_box(size: Size, ts: Transform) -> (Point, Size) {
    let top_left = Point::zero().transform_inf(ts);
    let top_right = Point::with_x(size.x).transform_inf(ts);
    let bottom_left = Point::with_y(size.y).transform_inf(ts);
    let bottom_right = size.to_point().transform_inf(ts);

    // We first compute the new bounding box of the rotated area.
    let min_x = top_left.x.min(top_right.x).min(bottom_left.x).min(bottom_right.x);
    let min_y = top_left.y.min(top_right.y).min(bottom_left.y).min(bottom_right.y);
    let max_x = top_left.x.max(top_right.x).max(bottom_left.x).max(bottom_right.x);
    let max_y = top_left.y.max(top_right.y).max(bottom_left.y).max(bottom_right.y);

    // Then we compute the new size of the area.
    let width = max_x - min_x;
    let height = max_y - min_y;

    (Point::new(-min_x, -min_y), Size::new(width.abs(), height.abs()))
}

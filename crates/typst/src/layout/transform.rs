use std::cell::LazyCell;
use std::ops::Div;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Content, NativeElement, Packed, Resolve, Show, Smart, StyleChain,
};
use crate::introspection::Locator;
use crate::layout::{
    layout_frame, Abs, Alignment, Angle, Axes, BlockElem, FixedAlignment, Frame,
    HAlignment, Length, Point, Ratio, Region, Rel, Size, VAlignment,
};
use crate::utils::Numeric;

/// Moves content without affecting layout.
///
/// The `move` function allows you to move content while the layout still 'sees'
/// it at the original positions. Containers will still be sized as if the
/// content was not moved.
///
/// # Example
/// ```example
/// #rect(inset: 0pt, move(
///   dx: 6pt, dy: 6pt,
///   rect(
///     inset: 8pt,
///     fill: white,
///     stroke: black,
///     [Abra cadabra]
///   )
/// ))
/// ```
#[elem(Show)]
pub struct MoveElem {
    /// The horizontal displacement of the content.
    pub dx: Rel<Length>,

    /// The vertical displacement of the content.
    pub dy: Rel<Length>,

    /// The content to move.
    #[required]
    pub body: Content,
}

impl Show for Packed<MoveElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_move)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the moved content.
#[typst_macros::time(span = elem.span())]
fn layout_move(
    elem: &Packed<MoveElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let mut frame = layout_frame(engine, &elem.body, locator, styles, region)?;
    let delta = Axes::new(elem.dx(styles), elem.dy(styles)).resolve(styles);
    let delta = delta.zip_map(region.size, Rel::relative_to);
    frame.translate(delta.to_point());
    Ok(frame)
}

/// Rotates content without affecting layout.
///
/// Rotates an element by a given angle. The layout will act as if the element
/// was not rotated unless you specify `{reflow: true}`.
///
/// # Example
/// ```example
/// #stack(
///   dir: ltr,
///   spacing: 1fr,
///   ..range(16)
///     .map(i => rotate(24deg * i)[X]),
/// )
/// ```
#[elem(Show)]
pub struct RotateElem {
    /// The amount of rotation.
    ///
    /// ```example
    /// #rotate(-1.571rad)[Space!]
    /// ```
    ///
    #[positional]
    pub angle: Angle,

    /// The origin of the rotation.
    ///
    /// If, for instance, you wanted the bottom left corner of the rotated
    /// element to stay aligned with the baseline, you would set it to `bottom +
    /// left` instead.
    ///
    /// ```example
    /// #set text(spacing: 8pt)
    /// #let square = square.with(width: 8pt)
    ///
    /// #box(square())
    /// #box(rotate(30deg, origin: center, square()))
    /// #box(rotate(30deg, origin: top + left, square()))
    /// #box(rotate(30deg, origin: bottom + right, square()))
    /// ```
    #[fold]
    #[default(HAlignment::Center + VAlignment::Horizon)]
    pub origin: Alignment,

    /// Whether the rotation impacts the layout.
    ///
    /// If set to `{false}`, the rotated content will retain the bounding box of
    /// the original content. If set to `{true}`, the bounding box will take the
    /// rotation of the content into account and adjust the layout accordingly.
    ///
    /// ```example
    /// Hello #rotate(90deg, reflow: true)[World]!
    /// ```
    #[default(false)]
    pub reflow: bool,

    /// The content to rotate.
    #[required]
    pub body: Content,
}

impl Show for Packed<RotateElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_rotate)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the rotated content.
#[typst_macros::time(span = elem.span())]
fn layout_rotate(
    elem: &Packed<RotateElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let angle = elem.angle(styles);
    let align = elem.origin(styles).resolve(styles);

    // Compute the new region's approximate size.
    let size = if region.size.is_finite() {
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
        elem.body(),
        Transform::rotate(angle),
        align,
        elem.reflow(styles),
    )
}

/// Scales content without affecting layout.
///
/// Lets you mirror content by specifying a negative scale on a single axis.
///
/// # Example
/// ```example
/// #set align(center)
/// #scale(x: -100%)[This is mirrored.]
/// #scale(x: -100%, reflow: true)[This is mirrored.]
/// ```
#[elem(Show)]
pub struct ScaleElem {
    /// The scaling factor for both axes, as a positional argument. This is just
    /// an optional shorthand notation for setting `x` and `y` to the same
    /// value.
    #[external]
    #[positional]
    #[default(Smart::Custom(ScaleAmount::Ratio(Ratio::one())))]
    pub factor: Smart<ScaleAmount>,

    /// The horizontal scaling factor.
    ///
    /// The body will be mirrored horizontally if the parameter is negative.
    #[parse(
        let all = args.find()?;
        args.named("x")?.or(all)
    )]
    #[default(Smart::Custom(ScaleAmount::Ratio(Ratio::one())))]
    pub x: Smart<ScaleAmount>,

    /// The vertical scaling factor.
    ///
    /// The body will be mirrored vertically if the parameter is negative.
    #[parse(args.named("y")?.or(all))]
    #[default(Smart::Custom(ScaleAmount::Ratio(Ratio::one())))]
    pub y: Smart<ScaleAmount>,

    /// The origin of the transformation.
    ///
    /// ```example
    /// A#box(scale(75%)[A])A \
    /// B#box(scale(75%, origin: bottom + left)[B])B
    /// ```
    #[fold]
    #[default(HAlignment::Center + VAlignment::Horizon)]
    pub origin: Alignment,

    /// Whether the scaling impacts the layout.
    ///
    /// If set to `{false}`, the scaled content will be allowed to overlap
    /// other content. If set to `{true}`, it will compute the new size of
    /// the scaled content and adjust the layout accordingly.
    ///
    /// ```example
    /// Hello #scale(x: 20%, y: 40%, reflow: true)[World]!
    /// ```
    #[default(false)]
    pub reflow: bool,

    /// The content to scale.
    #[required]
    pub body: Content,
}

impl Show for Packed<ScaleElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_scale)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the scaled content.
#[typst_macros::time(span = elem.span())]
fn layout_scale(
    elem: &Packed<ScaleElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    // Compute the new region's approximate size.
    let scale = elem.resolve_scale(engine, locator.relayout(), region.size, styles)?;
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
        elem.body(),
        Transform::scale(scale.x, scale.y),
        elem.origin(styles).resolve(styles),
        elem.reflow(styles),
    )
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum ScaleAmount {
    Ratio(Ratio),
    Length(Length),
}

impl Packed<ScaleElem> {
    /// Resolves scale parameters, preserving aspect ratio if one of the scales is set to `auto`.
    fn resolve_scale(
        &self,
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
                        Ratio::new(length.div(body()?))
                    }
                }),
            })
        }

        let size = LazyCell::new(|| {
            let pod = Region::new(container, Axes::splat(false));
            let frame = layout_frame(engine, &self.body, locator, styles, pod)?;
            SourceResult::Ok(frame.size())
        });

        let x = resolve_axis(
            self.x(styles),
            || size.as_ref().map(|size| size.x).map_err(Clone::clone),
            styles,
        )?;

        let y = resolve_axis(
            self.y(styles),
            || size.as_ref().map(|size| size.y).map_err(Clone::clone),
            styles,
        )?;

        match (x, y) {
            (Smart::Auto, Smart::Auto) => {
                bail!(self.span(), "x and y cannot both be auto")
            }
            (Smart::Custom(x), Smart::Custom(y)) => Ok(Axes::new(x, y)),
            (Smart::Auto, Smart::Custom(v)) | (Smart::Custom(v), Smart::Auto) => {
                Ok(Axes::splat(v))
            }
        }
    }
}

cast! {
    ScaleAmount,
    self => match self {
        ScaleAmount::Ratio(ratio) => ratio.into_value(),
        ScaleAmount::Length(length) => length.into_value(),
    },
    ratio: Ratio => ScaleAmount::Ratio(ratio),
    length: Length => ScaleAmount::Length(length),
}

/// Skews content.
///
/// Skews an element in horizontal and/or vertical direction. The layout will
/// act as if the element was not skewed unless you specify `{reflow: true}`.
///
/// # Example
/// ```example
/// #skew(ax: -12deg)[
///   This is some fake italic text.
/// ]
/// ```
#[elem(Show)]
pub struct SkewElem {
    /// The horizontal skewing angle.
    ///
    /// ```example
    /// #skew(ax: 30deg)[Skewed]
    /// ```
    ///
    #[default(Angle::zero())]
    pub ax: Angle,

    /// The vertical skewing angle.
    ///
    /// ```example
    /// #skew(ay: 30deg)[Skewed]
    /// ```
    ///
    #[default(Angle::zero())]
    pub ay: Angle,

    /// The origin of the skew transformation.
    ///
    /// The origin will stay fixed during the operation.
    ///
    /// ```example
    /// X #box(skew(ax: -30deg, origin: center + horizon)[X]) X \
    /// X #box(skew(ax: -30deg, origin: bottom + left)[X]) X \
    /// X #box(skew(ax: -30deg, origin: top + right)[X]) X
    /// ```
    #[fold]
    #[default(HAlignment::Center + VAlignment::Horizon)]
    pub origin: Alignment,

    /// Whether the skew transformation impacts the layout.
    ///
    /// If set to `{false}`, the skewed content will retain the bounding box of
    /// the original content. If set to `{true}`, the bounding box will take the
    /// transformation of the content into account and adjust the layout accordingly.
    ///
    /// ```example
    /// Hello #skew(ay: 30deg, reflow: true, "World")!
    /// ```
    #[default(false)]
    pub reflow: bool,

    /// The content to skew.
    #[required]
    pub body: Content,
}

impl Show for Packed<SkewElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_skew)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the skewed content.
#[typst_macros::time(span = elem.span())]
fn layout_skew(
    elem: &Packed<SkewElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    let ax = elem.ax(styles);
    let ay = elem.ay(styles);
    let align = elem.origin(styles).resolve(styles);

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
        elem.body(),
        Transform::skew(ax, ay),
        align,
        elem.reflow(styles),
    )
}

/// A scale-skew-translate transformation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Transform {
    pub sx: Ratio,
    pub ky: Ratio,
    pub kx: Ratio,
    pub sy: Ratio,
    pub tx: Abs,
    pub ty: Abs,
}

impl Transform {
    /// The identity transformation.
    pub const fn identity() -> Self {
        Self {
            sx: Ratio::one(),
            ky: Ratio::zero(),
            kx: Ratio::zero(),
            sy: Ratio::one(),
            tx: Abs::zero(),
            ty: Abs::zero(),
        }
    }

    /// A translate transform.
    pub const fn translate(tx: Abs, ty: Abs) -> Self {
        Self { tx, ty, ..Self::identity() }
    }

    /// A scale transform.
    pub const fn scale(sx: Ratio, sy: Ratio) -> Self {
        Self { sx, sy, ..Self::identity() }
    }

    /// A rotate transform.
    pub fn rotate(angle: Angle) -> Self {
        let cos = Ratio::new(angle.cos());
        let sin = Ratio::new(angle.sin());
        Self {
            sx: cos,
            ky: sin,
            kx: -sin,
            sy: cos,
            ..Self::default()
        }
    }

    /// A skew transform.
    pub fn skew(ax: Angle, ay: Angle) -> Self {
        Self {
            kx: Ratio::new(ax.tan()),
            ky: Ratio::new(ay.tan()),
            ..Self::identity()
        }
    }

    /// Whether this is the identity transformation.
    pub fn is_identity(self) -> bool {
        self == Self::identity()
    }

    /// Pre-concatenate another transformation.
    pub fn pre_concat(self, prev: Self) -> Self {
        Transform {
            sx: self.sx * prev.sx + self.kx * prev.ky,
            ky: self.ky * prev.sx + self.sy * prev.ky,
            kx: self.sx * prev.kx + self.kx * prev.sy,
            sy: self.ky * prev.kx + self.sy * prev.sy,
            tx: self.sx.of(prev.tx) + self.kx.of(prev.ty) + self.tx,
            ty: self.ky.of(prev.tx) + self.sy.of(prev.ty) + self.ty,
        }
    }

    /// Post-concatenate another transformation.
    pub fn post_concat(self, next: Self) -> Self {
        next.pre_concat(self)
    }

    /// Inverts the transformation.
    ///
    /// Returns `None` if the determinant of the matrix is zero.
    pub fn invert(self) -> Option<Self> {
        // Allow the trivial case to be inlined.
        if self.is_identity() {
            return Some(self);
        }

        // Fast path for scale-translate-only transforms.
        if self.kx.is_zero() && self.ky.is_zero() {
            if self.sx.is_zero() || self.sy.is_zero() {
                return Some(Self::translate(-self.tx, -self.ty));
            }

            let inv_x = 1.0 / self.sx;
            let inv_y = 1.0 / self.sy;
            return Some(Self {
                sx: Ratio::new(inv_x),
                ky: Ratio::zero(),
                kx: Ratio::zero(),
                sy: Ratio::new(inv_y),
                tx: -self.tx * inv_x,
                ty: -self.ty * inv_y,
            });
        }

        let det = self.sx * self.sy - self.kx * self.ky;
        if det.get().abs() < 1e-12 {
            return None;
        }

        let inv_det = 1.0 / det;
        Some(Self {
            sx: (self.sy * inv_det),
            ky: (-self.ky * inv_det),
            kx: (-self.kx * inv_det),
            sy: (self.sx * inv_det),
            tx: Abs::pt(
                (self.kx.get() * self.ty.to_pt() - self.sy.get() * self.tx.to_pt())
                    * inv_det,
            ),
            ty: Abs::pt(
                (self.ky.get() * self.tx.to_pt() - self.sx.get() * self.ty.to_pt())
                    * inv_det,
            ),
        })
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
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
        let frame = layout_frame(engine, body, locator.relayout(), styles, pod)?;

        // Actually perform the layout.
        let pod = Region::new(frame.size(), Axes::splat(true));
        let mut frame = layout_frame(engine, body, locator, styles, pod)?;
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
        let mut frame = layout_frame(engine, body, locator, styles, region)?;
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

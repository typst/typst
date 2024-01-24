use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, Packed, Resolve, StyleChain};
use crate::layout::{
    Abs, Alignment, Angle, Axes, FixedAlignment, Frame, HAlignment, LayoutMultiple,
    LayoutSingle, Length, Point, Ratio, Regions, Rel, Size, VAlignment,
};

/// Moves content without affecting layout.
///
/// The `move` function allows you to move content while th layout still 'sees'
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
#[elem(LayoutSingle)]
pub struct MoveElem {
    /// The horizontal displacement of the content.
    pub dx: Rel<Length>,

    /// The vertical displacement of the content.
    pub dy: Rel<Length>,

    /// The content to move.
    #[required]
    pub body: Content,
}

impl LayoutSingle for Packed<MoveElem> {
    #[typst_macros::time(name = "move", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        let pod = Regions::one(regions.base(), Axes::splat(false));
        let mut frame = self.body().layout(engine, styles, pod)?.into_frame();
        let delta = Axes::new(self.dx(styles), self.dy(styles)).resolve(styles);
        let delta = delta.zip_map(regions.base(), Rel::relative_to);
        frame.translate(delta.to_point());
        Ok(frame)
    }
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
#[elem(LayoutSingle)]
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

impl LayoutSingle for Packed<RotateElem> {
    #[typst_macros::time(name = "rotate", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        let angle = self.angle(styles);
        let align = self.origin(styles).resolve(styles);

        // Compute the new region's approximate size.
        let size = regions
            .base()
            .to_point()
            .transform_inf(Transform::rotate(angle))
            .map(Abs::abs)
            .to_size();

        measure_and_layout(
            engine,
            regions.base(),
            size,
            styles,
            self.body(),
            Transform::rotate(angle),
            align,
            self.reflow(styles),
        )
    }
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
#[elem(LayoutSingle)]
pub struct ScaleElem {
    /// The horizontal scaling factor.
    ///
    /// The body will be mirrored horizontally if the parameter is negative.
    #[parse(
        let all = args.find()?;
        args.named("x")?.or(all)
    )]
    #[default(Ratio::one())]
    pub x: Ratio,

    /// The vertical scaling factor.
    ///
    /// The body will be mirrored vertically if the parameter is negative.
    #[parse(args.named("y")?.or(all))]
    #[default(Ratio::one())]
    pub y: Ratio,

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

impl LayoutSingle for Packed<ScaleElem> {
    #[typst_macros::time(name = "scale", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        let sx = self.x(styles);
        let sy = self.y(styles);
        let align = self.origin(styles).resolve(styles);

        // Compute the new region's approximate size.
        let size = regions
            .base()
            .zip_map(Axes::new(sx, sy), |r, s| s.of(r))
            .map(Abs::abs);

        measure_and_layout(
            engine,
            regions.base(),
            size,
            styles,
            self.body(),
            Transform::scale(sx, sy),
            align,
            self.reflow(styles),
        )
    }
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
    base_size: Size,
    size: Size,
    styles: StyleChain,
    body: &Content,
    transform: Transform,
    align: Axes<FixedAlignment>,
    reflow: bool,
) -> SourceResult<Frame> {
    if !reflow {
        // Layout the body.
        let pod = Regions::one(base_size, Axes::splat(false));
        let mut frame = body.layout(engine, styles, pod)?.into_frame();
        let Axes { x, y } = align.zip_map(frame.size(), FixedAlignment::position);

        // Apply the transform.
        let ts = Transform::translate(x, y)
            .pre_concat(transform)
            .pre_concat(Transform::translate(-x, -y));
        frame.transform(ts);

        return Ok(frame);
    }

    // Measure the size of the body.
    let pod = Regions::one(size, Axes::splat(false));
    let frame = body.measure(engine, styles, pod)?.into_frame();

    // Actually perform the layout.
    let pod = Regions::one(frame.size(), Axes::splat(true));
    let mut frame = body.layout(engine, styles, pod)?.into_frame();
    let Axes { x, y } = align.zip_map(frame.size(), FixedAlignment::position);

    // Apply the transform.
    let ts = Transform::translate(x, y)
        .pre_concat(transform)
        .pre_concat(Transform::translate(-x, -y));

    // Compute the bounding box and offset and wrap in a new frame.
    let (offset, size) = compute_bounding_box(&frame, ts);
    frame.transform(ts);
    frame.translate(offset);
    frame.set_size(size);
    Ok(frame)
}

/// Computes the bounding box and offset of a transformed frame.
fn compute_bounding_box(frame: &Frame, ts: Transform) -> (Point, Size) {
    let top_left = Point::zero().transform_inf(ts);
    let top_right = Point::new(frame.width(), Abs::zero()).transform_inf(ts);
    let bottom_left = Point::new(Abs::zero(), frame.height()).transform_inf(ts);
    let bottom_right = Point::new(frame.width(), frame.height()).transform_inf(ts);

    // We first compute the new bounding box of the rotated frame.
    let min_x = top_left.x.min(top_right.x).min(bottom_left.x).min(bottom_right.x);
    let min_y = top_left.y.min(top_right.y).min(bottom_left.y).min(bottom_right.y);
    let max_x = top_left.x.max(top_right.x).max(bottom_left.x).max(bottom_right.x);
    let max_y = top_left.y.max(top_right.y).max(bottom_left.y).max(bottom_right.y);

    // Then we compute the new size of the frame.
    let width = max_x - min_x;
    let height = max_y - min_y;

    (Point::new(-min_x, -min_y), Size::new(width.abs(), height.abs()))
}

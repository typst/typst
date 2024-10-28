use crate::foundations::{cast, elem, Content, Func, Smart};
use crate::layout::{Abs, Angle, Length, Ratio, Rel};
use crate::math::Mathy;
use crate::visualize::Stroke;

/// Displays a diagonal line over a part of an equation.
///
/// This is commonly used to show the elimination of a term.
///
/// # Example
/// ```example
/// >>> #set page(width: 140pt)
/// Here, we can simplify:
/// $ (a dot b dot cancel(x)) /
///     cancel(x) $
/// ```
#[elem(Mathy)]
pub struct CancelElem {
    /// The content over which the line should be placed.
    #[required]
    pub body: Content,

    /// The length of the line, relative to the length of the diagonal spanning
    /// the whole element being "cancelled". A value of `{100%}` would then have
    /// the line span precisely the element's diagonal.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ a + cancel(x, length: #200%)
    ///     - cancel(x, length: #200%) $
    /// ```
    #[default(Rel::new(Ratio::one(), Abs::pt(3.0).into()))]
    pub length: Rel<Length>,

    /// Whether the cancel line should be inverted (flipped along the y-axis).
    /// For the default angle setting, inverted means the cancel line
    /// points to the top left instead of top right.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ (a cancel((b + c), inverted: #true)) /
    ///     cancel(b + c, inverted: #true) $
    /// ```
    #[default(false)]
    pub inverted: bool,

    /// Whether two opposing cancel lines should be drawn, forming a cross over
    /// the element. Overrides `inverted`.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(Pi, cross: #true) $
    /// ```
    #[default(false)]
    pub cross: bool,

    /// How much to rotate the cancel line.
    ///
    /// - If given an angle, the line is rotated by that angle clockwise with
    ///   respect to the y-axis.
    /// - If `{auto}`, the line assumes the default angle; that is, along the
    ///   rising diagonal of the content box.
    /// - If given a function `angle => angle`, the line is rotated, with
    ///   respect to the y-axis, by the angle returned by that function. The
    ///   function receives the default angle as its input.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(Pi)
    ///   cancel(Pi, angle: #0deg)
    ///   cancel(Pi, angle: #45deg)
    ///   cancel(Pi, angle: #90deg)
    ///   cancel(1/(1+x), angle: #(a => a + 45deg))
    ///   cancel(1/(1+x), angle: #(a => a + 90deg)) $
    /// ```
    pub angle: Smart<CancelAngle>,

    /// How to [stroke]($stroke) the cancel line.
    ///
    /// ```example
    /// >>> #set page(width: 140pt)
    /// $ cancel(
    ///   sum x,
    ///   stroke: #(
    ///     paint: red,
    ///     thickness: 1.5pt,
    ///     dash: "dashed",
    ///   ),
    /// ) $
    /// ```
    #[resolve]
    #[fold]
    #[default(Stroke {
        // Default stroke has 0.5pt for better visuals.
        thickness: Smart::Custom(Abs::pt(0.5).into()),
        ..Default::default()
    })]
    pub stroke: Stroke,
}

/// Defines the cancel line.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CancelAngle {
    Angle(Angle),
    Func(Func),
}

cast! {
    CancelAngle,
    self => match self {
        Self::Angle(v) => v.into_value(),
        Self::Func(v) => v.into_value()
    },
    v: Angle => CancelAngle::Angle(v),
    v: Func => CancelAngle::Func(v),
}

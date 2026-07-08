use std::hash::Hash;
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use typst_syntax::{Span, Spanned};
use typst_utils::{LazyHash, Numeric};

use crate::diag::{SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{Content, Repr, Resolve, Smart, StyleChain, func, scope, ty};
use crate::introspection::Locator;
use crate::layout::{Abs, Angle, Axes, Frame, Length, Region, Rel, Size};
use crate::visualize::RelativeTo;

/// A repeating tiling fill.
///
/// Typst supports the most common type of tilings, where a pattern is repeated
/// in a grid-like fashion, covering the entire area of an element that is
/// filled or stroked. The pattern is defined by a tile
/// @tiling.constructor.size[`size`] and a body defining the content of each
/// cell. You can also add horizontal or vertical
/// @tiling.constructor.spacing[`spacing`] between the cells of the tiling and
/// @tiling.constructor.offset[`offset`] and
/// @tiling.constructor.angle[`angle`] the starting placement of the tiling.
///
/// = Example <example>
/// ```example
/// #let pat = tiling(size: (30pt, 30pt), {
///   place(line(start: (0%, 0%), end: (100%, 100%)))
///   place(line(start: (0%, 100%), end: (100%, 0%)))
/// })
///
/// #rect(fill: pat, width: 100%, height: 60pt, stroke: 1pt)
/// ```
///
/// = Tilings on text <tilings-on-text>
/// Tilings are also supported on text, but only when setting
/// @tiling.constructor.relative[`relative`] to either `{auto}` (the default
/// value) or `{"parent"}`. To create word-by-word or glyph-by-glyph tilings,
/// you can wrap the words or characters of your text in @box[boxes] manually or
/// through a @reference:styling:show-rules[show rule].
///
/// ```example
/// #let pat = tiling(
///   size: (30pt, 30pt),
///   relative: "parent",
///   square(
///     size: 30pt,
///     fill: gradient
///       .conic(..color.map.rainbow),
///   )
/// )
///
/// #set text(fill: pat)
/// #lorem(10)
/// ```
#[ty(scope, cast, keywords = ["pattern"])]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Tiling(Arc<TilingInner>);

/// The internal representation of a [`Tiling`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct TilingInner {
    /// The tiling's rendered content.
    frame: LazyHash<Frame>,
    /// The tiling's tile size.
    size: Size,
    /// The tiling's tile spacing.
    spacing: Size,
    /// The tiling's tile offset.
    offset: Size,
    /// The tiling's tile angle.
    angle: Angle,
    /// The tiling's relative transform.
    relative: Smart<RelativeTo>,
}

#[scope]
#[expect(clippy::too_many_arguments)]
impl Tiling {
    /// Construct a new tiling.
    ///
    /// ```example
    /// #let pat = tiling(
    ///   size: (20pt, 20pt),
    ///   relative: "parent",
    ///   place(
    ///     dx: 5pt,
    ///     dy: 5pt,
    ///     rotate(45deg, square(
    ///       size: 5pt,
    ///       fill: black,
    ///     )),
    ///   ),
    /// )
    ///
    /// #rect(width: 100%, height: 60pt, fill: pat)
    /// ```
    #[func(constructor)]
    pub fn construct(
        engine: &mut Engine,
        span: Span,
        /// The bounding box of each cell of the tiling, specified as a `(x, y)`
        /// pair.
        ///
        /// If set to `{auto}`, the tiling takes on the size of the laid-out
        /// content.
        #[named]
        #[default(Spanned::detached(Smart::Auto))]
        size: Spanned<Smart<Axes<Length>>>,
        /// The spacing between cells of the tiling, specified as a `(x, y)`
        /// pair.
        ///
        /// If the spacing is lower than the size of the tiling, the tiling will
        /// overlap with itself. If it is higher, the tiling will have gaps.
        ///
        /// ```example
        /// >>> #set page(width: 5 * 30pt + 4 * 10pt + 2 * 15pt)
        /// #let pat = tiling(
        ///   size: (30pt, 30pt),
        ///   spacing: (10pt, 20pt),
        ///   square(size: 30pt, fill: gradient.conic(..color.map.rainbow)),
        /// )
        ///
        /// #rect(
        ///   width: 100%,
        ///   height: 80pt,
        ///   fill: pat,
        ///   stroke: (thickness: 1pt, dash: "dotted"),
        /// )
        /// ```
        #[named]
        #[default(Spanned::detached(Axes::splat(Length::zero())))]
        spacing: Spanned<Axes<Length>>,
        /// Shifts the entire tile grid without affecting the tile size or
        /// spacing.
        ///
        /// The offset is specified as a `(x, y)` pair. Positive `x` values move
        /// the pattern to the right and positive `y` values move it down.
        /// Relative values are resolved against the tile size plus spacing.
        ///
        /// Note that the displacement caused by the offset affects the tiles
        /// themselves while displacement of the inner contents (e.g. via
        /// `{place(dx: .., dy: ..)}`) can cause clipping when the content
        /// moves outside of the tile's bounding box.
        ///
        /// ```example
        /// #set rect(width: 100%, height: 80pt, stroke: 1pt)
        ///
        /// #let pat = tiling(
        ///   size: (20pt, 20pt),
        ///   circle(radius: 10pt, fill: blue),
        /// )
        ///
        /// #let pat-with-offset = tiling(
        ///   size: (20pt, 20pt),
        ///   offset: (50%, 50%),
        ///   circle(radius: 10pt, fill: blue),
        /// )
        ///
        /// #grid(
        ///   columns: 2,
        ///   column-gutter: 10pt,
        ///   rect(fill: pat),
        ///   rect(fill: pat-with-offset),
        /// )
        /// ```
        #[named]
        #[default(Spanned::new(Axes::splat(Rel::zero()), Span::detached()))]
        offset: Spanned<Axes<Rel<Length>>>,
        /// Rotates the tiles and the grid clockwise about the origin of the tiling.
        ///
        /// ```example
        /// #let pat = tiling(
        ///   size: (20pt, 20pt),
        ///   angle: 45deg,
        ///   line(start: (0%, 50%), end: (100%, 50%)),
        /// )
        ///
        /// #rect(width: 100%, height: 60pt, fill: pat)
        /// ```
        #[named]
        #[default(Angle::zero())]
        angle: Angle,
        /// Determines relative to which element's bounding box the tiling is
        /// drawn.
        ///
        /// By default, tilings are drawn relative to the shape they are being
        /// painted on (`{"self"}`), unless the tiling is applied on text, in
        /// which case they are relative to the closest ancestor container
        /// (`{"parent"}`).
        ///
        /// The parent of an element is the innermost @box or @block that
        /// contains the element, or, if there is none, the page itself.
        ///
        /// ```example
        /// #let pat = tiling(
        ///   size: (20pt, 20pt),
        ///   spacing: (5pt, 5pt),
        ///   relative: "self",
        ///   circle(radius: 10pt, fill: teal),
        /// )
        ///
        /// #let pat-with-parent = tiling(
        ///   size: (20pt, 20pt),
        ///   spacing: (5pt, 5pt),
        ///   relative: "parent",
        ///   circle(radius: 10pt, fill: teal),
        /// )
        ///
        /// #set raw(lang: "typc")
        /// #table(
        ///   columns: (1fr, 1fr, 1fr),
        ///   rows: (auto, 80pt),
        ///   table.header(`"self"`, `"parent"`, `"parent"`),
        ///
        ///   // This one is local to the cell itself.
        ///   table.cell(fill: pat, none),
        ///
        ///   // These two are both page-relative, so the
        ///   // pattern is continous.
        ///   table.cell(fill: pat-with-parent, none),
        ///   table.cell(fill: pat-with-parent, none),
        /// )
        /// ```
        #[named]
        #[default(Smart::Auto)]
        relative: Smart<RelativeTo>,
        /// The content of each cell of the tiling.
        body: Content,
    ) -> SourceResult<Tiling> {
        let size_span = size.span;
        if let Smart::Custom(size) = size.v {
            // Ensure that sizes are absolute.
            if !size.x.em.is_zero() || !size.y.em.is_zero() {
                bail!(size_span, "tile size must be absolute");
            }

            // Ensure that sizes are non-zero and finite.
            if size.x.is_zero()
                || size.y.is_zero()
                || !size.x.is_finite()
                || !size.y.is_finite()
            {
                bail!(size_span, "tile size must be non-zero and non-infinite");
            }
        }

        // Ensure that spacing is absolute.
        if !spacing.v.x.em.is_zero() || !spacing.v.y.em.is_zero() {
            bail!(spacing.span, "tile spacing must be absolute");
        }

        // Ensure that spacing is finite.
        if !spacing.v.x.is_finite() || !spacing.v.y.is_finite() {
            bail!(spacing.span, "tile spacing must be finite");
        }

        // Ensure that offset is not font-relative.
        if !offset.v.x.abs.em.is_zero() || !offset.v.y.abs.em.is_zero() {
            bail!(offset.span, "tile offset must not be font-relative");
        }

        // Ensure that offset is finite.
        if !offset.v.x.rel.get().is_finite()
            || !offset.v.x.abs.is_finite()
            || !offset.v.y.rel.get().is_finite()
            || !offset.v.y.abs.is_finite()
        {
            bail!(offset.span, "tile offset must be finite");
        }

        if !angle.is_finite() {
            bail!(span, "tile angle must be finite");
        }

        // The size of the frame
        let size = size.v.map(|l| l.map(|a| a.abs));
        let region = size.unwrap_or_else(|| Axes::splat(Abs::inf()));

        // Layout the tiling.
        let locator = Locator::root();
        let styles = StyleChain::new(&engine.library.styles);
        let pod = Region::new(region, Axes::splat(false));
        let mut frame =
            (engine.library.routines.layout_frame)(engine, &body, locator, styles, pod)?;

        // Set the size of the frame if the size is enforced.
        if let Smart::Custom(size) = size {
            frame.set_size(size);
        }

        // Check that the frame is non-zero.
        if frame.width().is_zero() || frame.height().is_zero() {
            bail!(
                span, "tile size must be non-zero";
                hint: "try setting the size manually";
            );
        }

        let size = frame.size();
        let spacing = spacing.v.map(|l| l.abs);
        let offset = offset
            .v
            .map(|l| l.resolve(styles))
            .zip_map(size + spacing, Rel::relative_to);

        Ok(Self(Arc::new(TilingInner {
            size,
            frame: LazyHash::new(frame),
            spacing,
            offset,
            angle,
            relative,
        })))
    }
}

impl Tiling {
    /// Set the relative placement of the tiling.
    pub fn with_relative(mut self, relative: RelativeTo) -> Self {
        if let Some(this) = Arc::get_mut(&mut self.0) {
            this.relative = Smart::Custom(relative);
        } else {
            self.0 = Arc::new(TilingInner {
                relative: Smart::Custom(relative),
                ..self.0.as_ref().clone()
            });
        }

        self
    }

    /// Return the offset of the tiling in absolute units.
    pub fn offset(&self) -> Size {
        self.0.offset
    }

    /// Return the rotation angle of the tiling.
    pub fn angle(&self) -> Angle {
        self.0.angle
    }

    /// Return the frame of the tiling.
    pub fn frame(&self) -> &Frame {
        &self.0.frame
    }

    /// Return the size of the tiling in absolute units.
    pub fn size(&self) -> Size {
        self.0.size
    }

    /// Return the spacing of the tiling in absolute units.
    pub fn spacing(&self) -> Size {
        self.0.spacing
    }

    /// Returns the relative placement of the tiling.
    pub fn relative(&self) -> Smart<RelativeTo> {
        self.0.relative
    }

    /// Returns the relative placement of the tiling.
    pub fn unwrap_relative(&self, on_text: bool) -> RelativeTo {
        self.0.relative.unwrap_or_else(|| {
            if on_text { RelativeTo::Parent } else { RelativeTo::Self_ }
        })
    }
}

impl Repr for Tiling {
    fn repr(&self) -> EcoString {
        let mut out =
            eco_format!("tiling(({}, {})", self.0.size.x.repr(), self.0.size.y.repr());

        if !self.0.spacing.is_zero() {
            out.push_str(", spacing: (");
            out.push_str(&self.0.spacing.x.repr());
            out.push_str(", ");
            out.push_str(&self.0.spacing.y.repr());
            out.push(')');
        }

        if !self.0.offset.is_zero() {
            out.push_str(", offset: (");
            out.push_str(&self.0.offset.x.repr());
            out.push_str(", ");
            out.push_str(&self.0.offset.y.repr());
            out.push(')');
        }

        if self.0.angle != Angle::zero() {
            out.push_str(", angle: ");
            out.push_str(&self.0.angle.repr());
        }

        out.push_str(", ..)");

        out
    }
}

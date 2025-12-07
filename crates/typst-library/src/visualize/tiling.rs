use std::hash::Hash;
use std::sync::Arc;

use ecow::{EcoString, eco_format};
use typst_syntax::{Span, Spanned};
use typst_utils::{LazyHash, Numeric};

use crate::World;
use crate::diag::{SourceResult, bail};
use crate::engine::Engine;
use crate::foundations::{Content, Repr, Smart, StyleChain, func, scope, ty};
use crate::introspection::Locator;
use crate::layout::{Abs, Axes, Frame, Length, Region, Size};
use crate::visualize::RelativeTo;

/// A repeating tiling fill.
///
/// Typst supports the most common type of tilings, where a pattern is repeated
/// in a grid-like fashion, covering the entire area of an element that is
/// filled or stroked. The pattern is defined by a tile size and a body defining
/// the content of each cell. You can also add horizontal or vertical spacing
/// between the cells of the tiling.
///
/// # Examples
///
/// ```example
/// #let pat = tiling(size: (30pt, 30pt))[
///   #place(line(start: (0%, 0%), end: (100%, 100%)))
///   #place(line(start: (0%, 100%), end: (100%, 0%)))
/// ]
///
/// #rect(fill: pat, width: 100%, height: 60pt, stroke: 1pt)
/// ```
///
/// Tilings are also supported on text, but only when setting the
/// [relativeness]($tiling.relative) to either `{auto}` (the default value) or
/// `{"parent"}`. To create word-by-word or glyph-by-glyph tilings, you can
/// wrap the words or characters of your text in [boxes]($box) manually or
/// through a [show rule]($styling/#show-rules).
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
///
/// You can also space the elements further or closer apart using the
/// [`spacing`]($tiling.spacing) feature of the tiling. If the spacing
/// is lower than the size of the tiling, the tiling will overlap.
/// If it is higher, the tiling will have gaps of the same color as the
/// background of the tiling.
///
/// ```example
/// #let pat = tiling(
///   size: (30pt, 30pt),
///   spacing: (10pt, 10pt),
///   relative: "parent",
///   square(
///     size: 30pt,
///     fill: gradient
///      .conic(..color.map.rainbow),
///   ),
/// )
///
/// #rect(
///   width: 100%,
///   height: 60pt,
///   fill: pat,
/// )
/// ```
///
/// # Relativeness
/// The location of the starting point of the tiling is dependent on the
/// dimensions of a container. This container can either be the shape that it is
/// being painted on, or the closest surrounding container. This is controlled
/// by the `relative` argument of a tiling constructor. By default, tilings
/// are relative to the shape they are being painted on, unless the tiling is
/// applied on text, in which case they are relative to the closest ancestor
/// container.
///
/// Typst determines the ancestor container as follows:
/// - For shapes that are placed at the root/top level of the document, the
///   closest ancestor is the page itself.
/// - For other shapes, the ancestor is the innermost [`block`] or [`box`] that
///   contains the shape. This includes the boxes and blocks that are implicitly
///   created by show rules and elements. For example, a [`rotate`] will not
///   affect the parent of a gradient, but a [`grid`] will.
///
/// # Compatibility
/// This type used to be called `pattern`. The name remains as an alias, but is
/// deprecated since Typst 0.13.
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
    /// The tiling's relative transform.
    relative: Smart<RelativeTo>,
}

#[scope]
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
        /// The bounding box of each cell of the tiling.
        #[named]
        #[default(Spanned::new(Smart::Auto, Span::detached()))]
        size: Spanned<Smart<Axes<Length>>>,
        /// The spacing between cells of the tiling.
        #[named]
        #[default(Spanned::new(Axes::splat(Length::zero()), Span::detached()))]
        spacing: Spanned<Axes<Length>>,
        /// The [relative placement](#relativeness) of the tiling.
        ///
        /// For an element placed at the root/top level of the document, the
        /// parent is the page itself. For other elements, the parent is the
        /// innermost block, box, column, grid, or stack that contains the
        /// element.
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

        // The size of the frame
        let size = size.v.map(|l| l.map(|a| a.abs));
        let region = size.unwrap_or_else(|| Axes::splat(Abs::inf()));

        // Layout the tiling.
        let world = engine.world;
        let library = world.library();
        let locator = Locator::root();
        let styles = StyleChain::new(&library.styles);
        let pod = Region::new(region, Axes::splat(false));
        let mut frame =
            (engine.routines.layout_frame)(engine, &body, locator, styles, pod)?;

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

        Ok(Self(Arc::new(TilingInner {
            size: frame.size(),
            frame: LazyHash::new(frame),
            spacing: spacing.v.map(|l| l.abs),
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

        if self.0.spacing.is_zero() {
            out.push_str(", spacing: (");
            out.push_str(&self.0.spacing.x.repr());
            out.push_str(", ");
            out.push_str(&self.0.spacing.y.repr());
            out.push(')');
        }

        out.push_str(", ..)");

        out
    }
}

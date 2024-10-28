use std::hash::Hash;
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use typst_syntax::{Span, Spanned};
use typst_utils::{LazyHash, Numeric};

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, repr, scope, ty, Content, Smart, StyleChain};
use crate::introspection::Locator;
use crate::layout::{Abs, Axes, Frame, Length, Region, Size};
use crate::visualize::RelativeTo;
use crate::World;

/// A repeating pattern fill.
///
/// Typst supports the most common pattern type of tiled patterns, where a
/// pattern is repeated in a grid-like fashion, covering the entire area of an
/// element that is filled or stroked. The pattern is defined by a tile size and
/// a body defining the content of each cell. You can also add horizontal or
/// vertical spacing between the cells of the pattern.
///
/// # Examples
///
/// ```example
/// #let pat = pattern(size: (30pt, 30pt))[
///   #place(line(start: (0%, 0%), end: (100%, 100%)))
///   #place(line(start: (0%, 100%), end: (100%, 0%)))
/// ]
///
/// #rect(fill: pat, width: 100%, height: 60pt, stroke: 1pt)
/// ```
///
/// Patterns are also supported on text, but only when setting the
/// [relativeness]($pattern.relative) to either `{auto}` (the default value) or
/// `{"parent"}`. To create word-by-word or glyph-by-glyph patterns, you can
/// wrap the words or characters of your text in [boxes]($box) manually or
/// through a [show rule]($styling/#show-rules).
///
/// ```example
/// #let pat = pattern(
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
/// [`spacing`]($pattern.spacing) feature of the pattern. If the spacing
/// is lower than the size of the pattern, the pattern will overlap.
/// If it is higher, the pattern will have gaps of the same color as the
/// background of the pattern.
///
/// ```example
/// #let pat = pattern(
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
/// The location of the starting point of the pattern is dependent on the
/// dimensions of a container. This container can either be the shape that it is
/// being painted on, or the closest surrounding container. This is controlled
/// by the `relative` argument of a pattern constructor. By default, patterns
/// are relative to the shape they are being painted on, unless the pattern is
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
#[ty(scope, cast)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Pattern(Arc<Repr>);

/// Internal representation of [`Pattern`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Repr {
    /// The pattern's rendered content.
    frame: LazyHash<Frame>,
    /// The pattern's tile size.
    size: Size,
    /// The pattern's tile spacing.
    spacing: Size,
    /// The pattern's relative transform.
    relative: Smart<RelativeTo>,
}

#[scope]
impl Pattern {
    /// Construct a new pattern.
    ///
    /// ```example
    /// #let pat = pattern(
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
        /// The callsite span.
        span: Span,
        /// The bounding box of each cell of the pattern.
        #[named]
        #[default(Spanned::new(Smart::Auto, Span::detached()))]
        size: Spanned<Smart<Axes<Length>>>,
        /// The spacing between cells of the pattern.
        #[named]
        #[default(Spanned::new(Axes::splat(Length::zero()), Span::detached()))]
        spacing: Spanned<Axes<Length>>,
        /// The [relative placement](#relativeness) of the pattern.
        ///
        /// For an element placed at the root/top level of the document, the
        /// parent is the page itself. For other elements, the parent is the
        /// innermost block, box, column, grid, or stack that contains the
        /// element.
        #[named]
        #[default(Smart::Auto)]
        relative: Smart<RelativeTo>,
        /// The content of each cell of the pattern.
        body: Content,
    ) -> SourceResult<Pattern> {
        let size_span = size.span;
        if let Smart::Custom(size) = size.v {
            // Ensure that sizes are absolute.
            if !size.x.em.is_zero() || !size.y.em.is_zero() {
                bail!(size_span, "pattern tile size must be absolute");
            }

            // Ensure that sizes are non-zero and finite.
            if size.x.is_zero()
                || size.y.is_zero()
                || !size.x.is_finite()
                || !size.y.is_finite()
            {
                bail!(size_span, "pattern tile size must be non-zero and non-infinite");
            }
        }

        // Ensure that spacing is absolute.
        if !spacing.v.x.em.is_zero() || !spacing.v.y.em.is_zero() {
            bail!(spacing.span, "pattern tile spacing must be absolute");
        }

        // Ensure that spacing is finite.
        if !spacing.v.x.is_finite() || !spacing.v.y.is_finite() {
            bail!(spacing.span, "pattern tile spacing must be finite");
        }

        // The size of the frame
        let size = size.v.map(|l| l.map(|a| a.abs));
        let region = size.unwrap_or_else(|| Axes::splat(Abs::inf()));

        // Layout the pattern.
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
                span, "pattern tile size must be non-zero";
                hint: "try setting the size manually"
            );
        }

        Ok(Self(Arc::new(Repr {
            size: frame.size(),
            frame: LazyHash::new(frame),
            spacing: spacing.v.map(|l| l.abs),
            relative,
        })))
    }
}

impl Pattern {
    /// Set the relative placement of the pattern.
    pub fn with_relative(mut self, relative: RelativeTo) -> Self {
        if let Some(this) = Arc::get_mut(&mut self.0) {
            this.relative = Smart::Custom(relative);
        } else {
            self.0 = Arc::new(Repr {
                relative: Smart::Custom(relative),
                ..self.0.as_ref().clone()
            });
        }

        self
    }

    /// Return the frame of the pattern.
    pub fn frame(&self) -> &Frame {
        &self.0.frame
    }

    /// Return the size of the pattern in absolute units.
    pub fn size(&self) -> Size {
        self.0.size
    }

    /// Return the spacing of the pattern in absolute units.
    pub fn spacing(&self) -> Size {
        self.0.spacing
    }

    /// Returns the relative placement of the pattern.
    pub fn relative(&self) -> Smart<RelativeTo> {
        self.0.relative
    }

    /// Returns the relative placement of the pattern.
    pub fn unwrap_relative(&self, on_text: bool) -> RelativeTo {
        self.0.relative.unwrap_or_else(|| {
            if on_text {
                RelativeTo::Parent
            } else {
                RelativeTo::Self_
            }
        })
    }
}

impl repr::Repr for Pattern {
    fn repr(&self) -> EcoString {
        let mut out =
            eco_format!("pattern(({}, {})", self.0.size.x.repr(), self.0.size.y.repr());

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

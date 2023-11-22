use std::hash::Hash;

use comemo::Prehashed;
use typst_syntax::{Span, Spanned};

use super::*;
use crate::diag::SourceResult;
use crate::doc::Frame;
use crate::eval::{scope, ty, Vm};
use crate::model::Content;
use crate::World;

/// A repeating pattern fill.
///
/// Typst supports the most common pattern type of tiled patterns, where a
/// pattern is repeated in a grid-like fashion. The pattern is defined by a
/// body and a tile size. The tile size is the size of each cell of the pattern.
/// The body is the content of each cell of the pattern. The pattern is
/// repeated in a grid-like fashion covering the entire area of the element
/// being filled. You can also specify a spacing between the cells of the
/// pattern, which is defined by a horizontal and vertical spacing. The spacing
/// is the distance between the edges of adjacent cells of the pattern. The default
/// spacing is zero.
///
/// # Examples
///
/// ```example
/// #let pat = pattern((30pt, 30pt))[
///   #place(top + left, line(start: (0%, 0%), end: (100%, 100%), stroke: 1pt))
///   #place(top + left, line(start: (0%, 100%), end: (100%, 0%), stroke: 1pt))
/// ]
///
/// #rect(fill: pat, width: 100%, height: 100%, stroke: 1pt)
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
///   (30pt, 30pt),
///   relative: "parent",
///   square(size: 30pt, fill: gradient.conic(..color.map.rainbow))
/// );
///  #set text(fill: pat)
///  #lorem(10)
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
///   (30pt, 30pt),
///   spacing: (10pt, 10pt),
///   relative: "parent",
///   square(size: 30pt, fill: gradient.conic(..color.map.rainbow))
/// );
///  #rect(width: 100%, height: 100%, fill: pat)
/// ```
///
/// # Relativeness
/// The location of the starting point of the pattern is dependant on the
/// dimensions of a container. This container can either be the shape they
/// are painted on, or the closest surrounding container. This is controlled by
/// the `relative` argument of a pattern constructor. By default, patterns are
/// relative to the shape they are painted on, unless the pattern is applied on
/// text, in which case they are relative to the closest ancestor container.
///
/// Typst determines the ancestor container as follows:
/// - For shapes that are placed at the root/top level of the document, the
///   closest ancestor is the page itself.
/// - For other shapes, the ancestor is the innermost [`block`]($block) or
///   [`box`]($box) that contains the shape. This includes the boxes and blocks
///   that are implicitly created by show rules and elements. For example, a
///   [`rotate`]($rotate) will not affect the parent of a gradient, but a
///   [`grid`]($grid) will.
#[ty(scope)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Pattern {
    /// The body of the pattern
    pub body: Prehashed<Content>,
    /// The pattern's rendered content.
    pub frame: Prehashed<Frame>,
    /// The pattern's tile size.
    pub bbox: Size,
    /// The pattern's tile spacing.
    pub spacing: Size,
    /// The pattern's relative transform.
    pub relative: Smart<Relative>,
}

impl Eq for Pattern {}

#[scope]
impl Pattern {
    /// Construct a new pattern.
    ///
    /// ```example
    /// #let pat = pattern(
    ///   (20pt, 20pt),
    ///   relative: "parent",
    ///   align(center + horizon, rotate(45deg, square(size: 10pt)))
    /// );
    ///  #rect(width: 100%, height: 100%, fill: pat)
    /// ```
    #[func(constructor)]
    pub fn construct(
        vm: &mut Vm,
        /// The bounding box of each cell of the pattern.
        bbox: Spanned<Axes<Length>>,
        /// The content of each cell of the pattern.
        body: Content,
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
        relative: Smart<Relative>,
    ) -> SourceResult<Pattern> {
        // Ensure that sizes are absolute.
        if !bbox.v.x.em.is_zero() || !bbox.v.y.em.is_zero() {
            bail!(bbox.span, "pattern tile size must be absolute");
        }

        // Ensure that sizes are non-zero and finite.
        if bbox.v.x.is_zero()
            || bbox.v.y.is_zero()
            || !bbox.v.x.is_finite()
            || !bbox.v.y.is_finite()
        {
            bail!(bbox.span, "pattern tile size must be non-zero and non-infinite");
        }

        // Ensure that spacing is absolute.
        if !spacing.v.x.em.is_zero() || !spacing.v.y.em.is_zero() {
            bail!(spacing.span, "pattern tile spacing must be absolute");
        }

        // Ensure that spacing is finite.
        if !spacing.v.x.is_finite() || !spacing.v.y.is_finite() {
            bail!(spacing.span, "pattern tile spacing must be finite");
        }

        // The size of the pattern.
        let size = Size::new(bbox.v.x.abs, bbox.v.y.abs);

        // Layout the pattern.
        let library = vm.vt.world.library();
        let mut frame =
            (library.items.layout_one)(&mut vm.vt, &body, StyleChain::default(), size)?;

        // Ensure that the frame has the correct size.
        frame.set_size(size);

        Ok(Self {
            body: Prehashed::new(body),
            frame: Prehashed::new(frame),
            bbox: size,
            spacing: spacing.v.map(|l| l.abs),
            relative,
        })
    }

    /// Returns the content of an individual tile of the pattern.
    #[func]
    pub fn body(&self) -> Content {
        self.body.clone().into_inner()
    }

    /// Returns the size of an individual tile of the pattern.
    #[func]
    pub fn size(&self) -> Axes<Length> {
        self.bbox.map(|l| Length { abs: l, em: Em::zero() })
    }

    /// Returns the spacing between tiles of the pattern.
    #[func]
    pub fn spacing(&self) -> Axes<Length> {
        self.spacing.map(|l| Length { abs: l, em: Em::zero() })
    }

    /// Returns the relative placement of the pattern.
    #[func]
    pub fn relative(&self) -> Smart<Relative> {
        self.relative
    }
}

impl Pattern {
    pub fn with_relative(self, relative: Relative) -> Self {
        Self { relative: Smart::Custom(relative), ..self }
    }

    pub fn unwrap_relative(&self, on_text: bool) -> Relative {
        self.relative.unwrap_or_else(|| {
            if on_text {
                Relative::Parent
            } else {
                Relative::Self_
            }
        })
    }
}

impl Repr for Pattern {
    fn repr(&self) -> EcoString {
        todo!()
    }
}

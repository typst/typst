use std::hash::Hash;

use comemo::Prehashed;
use typst_syntax::{Span, Spanned};

use super::*;
use crate::diag::SourceResult;
use crate::doc::Frame;
use crate::eval::{scope, ty, Vm};
use crate::model::Content;
use crate::World;

#[ty(scope)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Pattern {
    /// The pattern's content.
    pub body: Prehashed<Frame>,
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
        let body = (library.items.layout_one)(
            &mut vm.vt,
            &Content::from(body),
            StyleChain::default(),
            size,
        )?;

        Ok(Self {
            body: Prehashed::new(body),
            bbox: size,
            spacing: spacing.v.map(|l| l.abs),
            relative,
        })
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

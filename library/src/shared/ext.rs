//! Extension traits.

use crate::layout::{AlignElem, MoveElem, PadElem};
use crate::prelude::*;
use crate::text::{EmphElem, FontFamily, FontList, StrongElem, TextElem, UnderlineElem};

/// Additional methods on content.
pub trait ContentExt {
    /// Make this content strong.
    #[must_use]
    fn strong(self) -> Self;

    /// Make this content emphasized.
    #[must_use]
    fn emph(self) -> Self;

    /// Underline this content.
    #[must_use]
    fn underlined(self) -> Self;

    /// Link the content somewhere.
    #[must_use]
    fn linked(self, dest: Destination) -> Self;

    /// Set alignments for this content.
    #[must_use]
    fn aligned(self, aligns: Axes<Option<GenAlign>>) -> Self;

    /// Pad this content at the sides.
    #[must_use]
    fn padded(self, padding: Sides<Rel<Length>>) -> Self;

    /// Transform this content's contents without affecting layout.
    #[must_use]
    fn moved(self, delta: Axes<Rel<Length>>) -> Self;
}

impl ContentExt for Content {
    #[inline]
    fn strong(self) -> Self {
        StrongElem::new(self).pack()
    }

    #[inline]
    fn emph(self) -> Self {
        EmphElem::new(self).pack()
    }

    #[inline]
    fn underlined(self) -> Self {
        UnderlineElem::new(self).pack()
    }

    #[inline]
    fn linked(self, dest: Destination) -> Self {
        self.styled(MetaElem::set_data(vec![Meta::Link(dest)]))
    }

    #[inline]
    fn aligned(self, aligns: Axes<Option<GenAlign>>) -> Self {
        self.styled(AlignElem::set_alignment(aligns))
    }

    #[inline]
    fn padded(self, padding: Sides<Rel<Length>>) -> Self {
        PadElem::new(self)
            .with_left(padding.left)
            .with_top(padding.top)
            .with_right(padding.right)
            .with_bottom(padding.bottom)
            .pack()
    }

    #[inline]
    fn moved(self, delta: Axes<Rel<Length>>) -> Self {
        MoveElem::new(self).with_dx(delta.x).with_dy(delta.y).pack()
    }
}

/// Additional methods for style lists.
pub trait StylesExt {
    /// Set a font family composed of a preferred family and existing families
    /// from a style chain.
    fn set_family(&mut self, preferred: FontFamily, existing: StyleChain<'_>);
}

impl StylesExt for Styles {
    #[inline]
    fn set_family(&mut self, preferred: FontFamily, existing: StyleChain<'_>) {
        self.set(TextElem::set_font(FontList(
            std::iter::once(preferred)
                .chain(TextElem::font_in(existing))
                .collect(),
        )));
    }
}

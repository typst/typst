//! Extension traits.

use crate::layout::{AlignElem, MoveElem, PadElem};
use crate::prelude::*;
use crate::text::{EmphElem, FontFamily, FontList, StrongElem, TextElem, UnderlineElem};

/// Additional methods on content.
pub trait ContentExt {
    /// Make this content strong.
    fn strong(self) -> Self;

    /// Make this content emphasized.
    fn emph(self) -> Self;

    /// Underline this content.
    fn underlined(self) -> Self;

    /// Link the content somewhere.
    fn linked(self, dest: Destination) -> Self;

    /// Make the content linkable by `.linked(Destination::Location(loc))`.
    ///
    /// Should be used in combination with [`Location::variant`].
    fn backlinked(self, loc: Location) -> Self;

    /// Set alignments for this content.
    fn aligned(self, align: Align) -> Self;

    /// Pad this content at the sides.
    fn padded(self, padding: Sides<Rel<Length>>) -> Self;

    /// Transform this content's contents without affecting layout.
    fn moved(self, delta: Axes<Rel<Length>>) -> Self;
}

impl ContentExt for Content {
    fn strong(self) -> Self {
        StrongElem::new(self).pack()
    }

    fn emph(self) -> Self {
        EmphElem::new(self).pack()
    }

    fn underlined(self) -> Self {
        UnderlineElem::new(self).pack()
    }

    fn linked(self, dest: Destination) -> Self {
        self.styled(MetaElem::set_data(vec![Meta::Link(dest)]))
    }

    fn backlinked(self, loc: Location) -> Self {
        let mut backlink = Content::empty();
        backlink.set_location(loc);
        self.styled(MetaElem::set_data(vec![Meta::Elem(backlink)]))
    }

    fn aligned(self, align: Align) -> Self {
        self.styled(AlignElem::set_alignment(align))
    }

    fn padded(self, padding: Sides<Rel<Length>>) -> Self {
        PadElem::new(self)
            .with_left(padding.left)
            .with_top(padding.top)
            .with_right(padding.right)
            .with_bottom(padding.bottom)
            .pack()
    }

    fn moved(self, delta: Axes<Rel<Length>>) -> Self {
        MoveElem::new(self).with_dx(delta.x).with_dy(delta.y).pack()
    }
}

/// Additional methods for style lists.
pub trait StylesExt {
    /// Set a font family composed of a preferred family and existing families
    /// from a style chain.
    fn set_family(&mut self, preferred: FontFamily, existing: StyleChain);
}

impl StylesExt for Styles {
    fn set_family(&mut self, preferred: FontFamily, existing: StyleChain) {
        self.set(TextElem::set_font(FontList(
            std::iter::once(preferred)
                .chain(TextElem::font_in(existing))
                .collect(),
        )));
    }
}

//! State needed for space collapsing

use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::TagElem;
use typst_library::layout::HElem;
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// State kept for space collapsing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SpaceState {
    /// Invisible elements do not impact space collapsing.
    Invisible,
    /// Destructive elements remove spaces that come before or after.
    Destructive,
    /// Normal elements. Spaces are only kept if supported on both sides.
    Supportive,
    /// Adjacent spaces collapse as one with the styles of the first space.
    Space,
}

/// Space collapsing state for general elements.
pub(crate) fn collapse_state(content: &Content, styles: StyleChain) -> SpaceState {
    // Roughly ordered from most to least common.
    if content.is::<TagElem>() {
        SpaceState::Invisible
    } else if content.is::<SpaceElem>() {
        SpaceState::Space
    } else if content.is::<LinebreakElem>() {
        SpaceState::Destructive
    } else if let Some(elem) = content.to_packed::<HElem>() {
        if elem.amount.is_fractional() || elem.weak.get(styles) {
            SpaceState::Destructive
        } else {
            SpaceState::Invisible
        }
    } else {
        SpaceState::Supportive
    }
}

/// Space collapsing state for textual elements used during regex matching.
pub(crate) fn collapse_state_textual<'a>(
    content: &'a Content,
    styles: StyleChain<'_>,
) -> (SpaceState, &'a str) {
    // Roughly ordered from most to least common.
    if content.is::<TagElem>() {
        (SpaceState::Invisible, "")
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        (SpaceState::Supportive, &elem.text)
    } else if let Some(elem) = content.to_packed::<SmartQuoteElem>() {
        let text = if elem.double.get(styles) { "\"" } else { "'" };
        (SpaceState::Supportive, text)
    } else if content.is::<SpaceElem>() {
        (SpaceState::Space, " ")
    } else if content.is::<LinebreakElem>() {
        (SpaceState::Destructive, "\n")
    } else {
        let name = content.elem().name();
        panic!("tried to find regex match in a non-textual element: {name}");
    }
}

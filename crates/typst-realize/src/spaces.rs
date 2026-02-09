//! The space collapsing infrastructure for realization.

use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::TagElem;
use typst_library::layout::HElem;
use typst_library::routines::Pair;
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// State kept for space collapsing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SpaceState {
    /// Invisible elements do not impact space collapsing.
    Invisible,
    /// Destructive elements discard spaces that come before or after.
    Destructive,
    /// Normal elements. Spaces are only kept if supported on both sides.
    Supportive,
    /// Adjacent spaces collapse as one with the styles of the first space.
    Space,
}

/// Run the space collapsing algorithm on `buf[start..]`. This discards space
/// elements that are at the edges of the range or in the vicinity of
/// destructive elements and collapses adjacent spaces into one with the styles
/// of the first space.
///
/// This is implemented efficiently in-place by shifting elements in the buffer
/// to the left whenever we discard or collapse a space.
pub(crate) fn collapse_spaces(buf: &mut Vec<Pair>, start: usize) {
    let mut cursor = start;
    let mut prev_space = cursor;
    let mut state = SpaceState::Destructive;

    // We do one pass over the elements, backshifting everything as necessary
    // when a space collapses. The variable `cursor` is our cursor in the
    // result. The variable `i` is our cursor in the original elements. At all
    // times, we have `cursor <= i`, so we can do it in-place.
    for i in start..buf.len() {
        let (content, styles) = buf[i];

        state = match collapse_state(content, styles) {
            SpaceState::Invisible => state,
            SpaceState::Destructive => {
                if state == SpaceState::Space {
                    buf.copy_within(prev_space + 1..cursor, prev_space);
                    cursor -= 1;
                }
                SpaceState::Destructive
            }
            SpaceState::Supportive => SpaceState::Supportive,
            SpaceState::Space => {
                if state != SpaceState::Supportive {
                    continue;
                }
                prev_space = cursor;
                SpaceState::Space
            }
        };

        // Copy over normal elements (in place).
        if cursor < i {
            buf[cursor] = buf[i];
        }
        cursor += 1;
    }

    if state == SpaceState::Space {
        buf.copy_within(prev_space + 1..cursor, prev_space);
        cursor -= 1;
    }

    // Delete all the excess that's left due to the gaps produced by spaces.
    buf.truncate(cursor);
}

/// Space collapsing state for general elements.
pub(crate) fn collapse_state(content: &Content, styles: StyleChain) -> SpaceState {
    if content.is::<TagElem>() {
        SpaceState::Invisible
    } else if let Some(elem) = content.to_packed::<HElem>() {
        if elem.amount.is_fractional() || elem.weak.get(styles) {
            SpaceState::Destructive
        } else {
            SpaceState::Invisible
        }
    } else if content.is::<LinebreakElem>() {
        SpaceState::Destructive
    } else if content.is::<SpaceElem>() {
        SpaceState::Space
    } else {
        SpaceState::Supportive
    }
}

/// Space collapsing state for textual elements used during regex matching.
pub(crate) fn collapse_state_textual<'a>(
    content: &'a Content,
    styles: StyleChain<'_>,
) -> (SpaceState, &'a str) {
    if content.is::<TagElem>() {
        (SpaceState::Invisible, "")
    } else if content.is::<LinebreakElem>() {
        (SpaceState::Destructive, "\n")
    } else if content.is::<SpaceElem>() {
        (SpaceState::Space, " ")
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        (SpaceState::Supportive, &elem.text)
    } else if let Some(elem) = content.to_packed::<SmartQuoteElem>() {
        let text = if elem.double.get(styles) { "\"" } else { "'" };
        (SpaceState::Supportive, text)
    } else {
        let name = content.elem().name();
        panic!("tried to find regex match in a non-textual element: {name}");
    }
}

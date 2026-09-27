//! The space collapsing and discarding infrastructure for realization.

use codex::space_discarding::discard_space_between;
use typst_html::HtmlElem;
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::TagElem;
use typst_library::layout::HElem;
use typst_library::routines::Pair;
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// State kept for space collapsing/discarding.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SpaceState<'a> {
    /// When destructive, we skip any future spaces.
    Destructive,
    /// When supportive, we usually keep future spaces, but we will discard
    /// newline spaces if the text surrounding a newline space returns true for
    /// [`discard_space_between`].
    ///
    /// Text can be empty if this is not a textual element. This is easier than
    /// using an `Option`.
    Supportive { text: &'a str },
    /// A current space. Spaces collapse into one with the styles of the first
    /// space by skipping following spaces, and they set `had_newline` to true
    /// if any space in a group had a newline.
    ///
    /// Since `had_newline` may update based on a future space, all spaces need
    /// to remember the previous text of a supportive element to do the
    /// [`discard_space_between`] check.
    ///
    /// The `had_newline` grouping is done so that spaces before line comments
    /// don't add new space. E.g. `x //\n`
    Space { prev_text: &'a str, had_newline: bool },
}

/// What action to take for space collapsing/discarding.
///
/// This is in addition to updating the [`SpaceState`] itself, which is
/// necessary even when the action is `Skip`.
#[derive(Debug, Copy, Clone)]
pub(crate) enum SpaceAction {
    /// Invisible elements are themselves kept, but neither contain text nor
    /// affect the space collapsing state.
    Invisible,
    /// Avoid adding the current space element.
    Skip,
    /// Discard the preceding space, but keep the current element.
    ///
    /// This is not returned unless there was a preceding space.
    Discard,
    /// Keep the current element and don't change any preceding spaces (if any).
    ///
    /// This is given for destructive elements that weren't preceded by a space.
    Keep,
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

        let action;
        (action, state) = collapse_transition(state, content, styles);
        match action {
            SpaceAction::Invisible => {}
            SpaceAction::Skip => continue,
            SpaceAction::Discard => {
                buf.copy_within(prev_space + 1..cursor, prev_space);
                cursor -= 1;
            }
            SpaceAction::Keep => {
                prev_space = cursor;
            }
        }

        // Copy over normal elements (in place).
        if cursor < i {
            buf[cursor] = buf[i];
        }
        cursor += 1;
    }

    if matches!(state, SpaceState::Space { .. }) {
        buf.copy_within(prev_space + 1..cursor, prev_space);
        cursor -= 1;
    }

    // Delete all the excess that's left due to the gaps produced by spaces.
    buf.truncate(cursor);
}

/// How to transition state for the space collapsing algorithm.
fn collapse_transition<'a>(
    state: SpaceState<'a>,
    content: &'a Content,
    styles: StyleChain<'_>,
) -> (SpaceAction, SpaceState<'a>) {
    if content.is::<TagElem>() {
        (SpaceAction::Invisible, state)
    } else if let Some(elem) = content.to_packed::<HElem>() {
        if elem.amount.is_fractional() || elem.weak.get(styles) {
            for_destructive(state)
        } else {
            (SpaceAction::Invisible, state)
        }
    } else if content.is::<LinebreakElem>()
        // We want to collapse spaces that would otherwise be protected and show
        // up as spans with `white-space: pre-wrap`.
        || content.to_packed::<HtmlElem>().is_some_and(|elem| {
            typst_html::tag::is_whitespace_collapsing(elem.tag)
        })
    {
        for_destructive(state)
    } else if let Some(elem) = content.to_packed::<SpaceElem>() {
        for_space(state, elem.had_newline)
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        for_supportive(state, &elem.text)
    } else {
        for_supportive(state, "")
    }
}

/// How to transition state for space collapsing during regex matching.
pub(crate) fn collapse_transition_textual<'a>(
    state: SpaceState<'a>,
    content: &'a Content,
    styles: StyleChain<'_>,
) -> (SpaceAction, SpaceState<'a>, &'a str) {
    let text;
    // Roughly ordered from most to least common.
    let (action, state) = if content.is::<TagElem>() {
        text = "";
        (SpaceAction::Invisible, state)
    } else if content.is::<LinebreakElem>() {
        text = "\n";
        for_destructive(state)
    } else if let Some(elem) = content.to_packed::<SpaceElem>() {
        text = " ";
        for_space(state, elem.had_newline)
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        text = &elem.text;
        for_supportive(state, text)
    } else if let Some(elem) = content.to_packed::<SmartQuoteElem>() {
        text = if elem.double.get(styles) { "\"" } else { "'" };
        for_supportive(state, text)
    } else {
        let name = content.elem().name();
        panic!("tried to find regex match in a non-textual element: {name}");
    };
    (action, state, text)
}

/// The state transition for a destructive element.
fn for_destructive(state: SpaceState) -> (SpaceAction, SpaceState) {
    if matches!(state, SpaceState::Space { .. }) {
        (SpaceAction::Discard, SpaceState::Destructive)
    } else {
        (SpaceAction::Keep, SpaceState::Destructive)
    }
}

/// The state transition for a space element.
///
/// If any space in a group of spaces had a newline, we treat all spaces in that
/// group as having a newline.
fn for_space(state: SpaceState, had_newline: bool) -> (SpaceAction, SpaceState) {
    match state {
        SpaceState::Destructive => (SpaceAction::Skip, SpaceState::Destructive),
        SpaceState::Supportive { text: prev_text } => {
            (SpaceAction::Keep, SpaceState::Space { prev_text, had_newline })
        }
        SpaceState::Space { prev_text, had_newline: prev_nl } => (
            SpaceAction::Skip,
            SpaceState::Space { prev_text, had_newline: prev_nl || had_newline },
        ),
    }
}

/// The state transition for a supportive element. `text` should be empty if
/// this is not a textual element.
fn for_supportive<'a>(
    state: SpaceState<'_>,
    text: &'a str,
) -> (SpaceAction, SpaceState<'a>) {
    if let SpaceState::Space { prev_text, had_newline } = state
        && had_newline
        && discard_space_between(prev_text, text)
    {
        (SpaceAction::Discard, SpaceState::Supportive { text })
    } else {
        (SpaceAction::Keep, SpaceState::Supportive { text })
    }
}

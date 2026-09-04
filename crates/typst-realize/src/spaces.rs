//! The space collapsing and discarding infrastructure for realization.
use codex::space_discarding::discard_space_between;

use typst_html::HtmlElem;
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::TagElem;
use typst_library::layout::HElem;
use typst_library::routines::Pair;
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// State kept for space collapsing/discarding.
///
/// We store the string of preceding text elements to delay the expensive
/// [`discard_space_between`] check until we encounter a newline space.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SpaceState<'a> {
    /// When destructive, we skip any future spaces.
    Destructive,
    /// When supportive, we usually keep future spaces, but we will skip newline
    /// spaces if our text ends in a space-discarding character.
    Supportive { text: Option<&'a str> },
    /// A current space that did not have a newline and remembers the preceding
    /// element's text to check if it was space-discarding.
    ///
    /// Skips future spaces and may itself be discarded if followed by a
    /// destructive element or followed by a newline space when the previous
    /// text ended space-discarding.
    Space { prev_text: Option<&'a str>, had_newline: bool },
}

impl SpaceState<'_> {
    /// Whether the current state is a space.
    fn is_space(self) -> bool {
        matches!(self, Self::Space { .. })
    }
}

/// What action to take for space collapsing.
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

    if state.is_space() {
        buf.copy_within(prev_space + 1..cursor, prev_space);
        cursor -= 1;
    }

    // Delete all the excess that's left due to the gaps produced by spaces.
    buf.truncate(cursor);
}

/// How to transition state for the space collapsing algorithm.
pub(crate) fn collapse_transition<'a>(
    state: SpaceState<'a>,
    content: &'a Content,
    styles: StyleChain<'_>,
) -> (SpaceAction, SpaceState<'a>) {
    if content.is::<TagElem>() {
        (SpaceAction::Invisible, state)
    } else if let Some(elem) = content.to_packed::<HElem>() {
        if elem.amount.is_fractional() || elem.weak.get(styles) {
            if state.is_space() {
                (SpaceAction::Discard, SpaceState::Destructive)
            } else {
                (SpaceAction::Keep, SpaceState::Destructive)
            }
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
        if state.is_space() {
            (SpaceAction::Discard, SpaceState::Destructive)
        } else {
            (SpaceAction::Keep, SpaceState::Destructive)
        }
    } else if let Some(elem) = content.to_packed::<SpaceElem>() {
        for_space(state, elem.had_newline)
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        for_text(state, &elem.text)
    } else {
        (SpaceAction::Keep, SpaceState::Supportive { text: None })
    }
}

/// How to transition state for space collapsing during regex matching.
pub(crate) fn collapse_transition_textual<'a>(
    state: SpaceState<'a>,
    content: &'a Content,
    styles: StyleChain<'_>,
) -> (SpaceAction, SpaceState<'a>, &'a str) {
    // Roughly ordered from most to least common.
    if content.is::<TagElem>() {
        (SpaceAction::Invisible, state, "")
    } else if content.is::<LinebreakElem>() {
        if state.is_space() {
            (SpaceAction::Discard, SpaceState::Destructive, "\n")
        } else {
            (SpaceAction::Keep, SpaceState::Destructive, "\n")
        }
    } else if let Some(elem) = content.to_packed::<SpaceElem>() {
        let (action, state) = for_space(state, elem.had_newline);
        (action, state, " ")
    } else if let Some(elem) = content.to_packed::<TextElem>() {
        let (action, state) = for_text(state, &elem.text);
        (action, state, &elem.text)
    } else if let Some(elem) = content.to_packed::<SmartQuoteElem>() {
        let text = if elem.double.get(styles) { "\"" } else { "'" };
        // `text: None` because this text isn't space-discarding.
        (SpaceAction::Keep, SpaceState::Supportive { text: None }, text)
    } else {
        let name = content.elem().name();
        panic!("tried to find regex match in a non-textual element: {name}");
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
        SpaceState::Space { prev_text, had_newline: mut prev_nl } => {
            prev_nl |= had_newline;
            (SpaceAction::Skip, SpaceState::Space { prev_text, had_newline: prev_nl })
        }
    }
}

/// The state transition for a text element.
fn for_text<'a>(state: SpaceState<'_>, text: &'a str) -> (SpaceAction, SpaceState<'a>) {
    let action = match state {
        SpaceState::Space { prev_text: Some(prev_text), had_newline: true }
            if discard_space_between(prev_text, text) =>
        {
            SpaceAction::Discard
        }
        _ => SpaceAction::Keep,
    };
    (action, SpaceState::Supportive { text: Some(text) })
}

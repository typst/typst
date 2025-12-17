//! The space collapsing and discarding infrastructure for realization.

use icu_properties::maps::CodePointMapDataBorrowed;
use icu_properties::sets::CodePointSetDataBorrowed;
use icu_properties::{EastAsianWidth, Script};

use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::TagElem;
use typst_library::layout::HElem;
use typst_library::routines::Pair;
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

/// State kept for space collapsing/discarding.
///
/// We store the string of preceding text elements to delay the expensive
/// [`is_space_discarding`] check until we encounter a newline space.
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
    Space { prev_text: Option<&'a str> },
    /// A current space that did have a newline.
    ///
    /// Does not need to store the preceding element's text, as this would have
    /// been skipped if that text ended as space-discarding.
    SpaceWithNewline,
}

/// What action to take for space collapsing.
///
/// This is in addition to updating the `SpaceState` itself, which is necessary
/// even when the action is `Skip`.
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
    /// Discard the preceding space and skip the current space element.
    ///
    /// This is not returned unless there was a preceding space.
    DiscardAndSkip,
    /// Keep the current element and don't change any preceding spaces (if any).
    ///
    /// This is given for destructive elements that weren't preceded by a space.
    Keep,
}

/// Whether the current state is a space.
fn is_space(state: SpaceState) -> bool {
    matches!(state, SpaceState::Space { .. } | SpaceState::SpaceWithNewline)
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
            SpaceAction::DiscardAndSkip => {
                buf.copy_within(prev_space + 1..cursor, prev_space);
                cursor -= 1;
                continue;
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

    if is_space(state) {
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
            if is_space(state) {
                (SpaceAction::Discard, SpaceState::Destructive)
            } else {
                (SpaceAction::Keep, SpaceState::Destructive)
            }
        } else {
            (SpaceAction::Invisible, state)
        }
    } else if content.is::<LinebreakElem>() {
        if is_space(state) {
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
        if is_space(state) {
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

/// The state transition for a text element.
fn for_text<'a>(state: SpaceState<'_>, text: &'a str) -> (SpaceAction, SpaceState<'a>) {
    if state == SpaceState::SpaceWithNewline
        && text.chars().next().is_some_and(is_space_discarding)
    {
        (SpaceAction::Discard, SpaceState::Supportive { text: Some(text) })
    } else {
        (SpaceAction::Keep, SpaceState::Supportive { text: Some(text) })
    }
}

/// The state transition for a space element.
///
/// Note that if any space in a group of spaces had a newline, we treat all
/// spaces in that group as having a newline.
fn for_space(state: SpaceState, had_nl: bool) -> (SpaceAction, SpaceState) {
    match state {
        // Destructive
        SpaceState::Destructive => (SpaceAction::Skip, SpaceState::Destructive),
        // Supportive
        SpaceState::Supportive { text: Some(text) }
            if had_nl && text.chars().next_back().is_some_and(is_space_discarding) =>
        {
            (SpaceAction::Skip, SpaceState::Destructive)
        }
        SpaceState::Supportive { .. } if had_nl => {
            (SpaceAction::Keep, SpaceState::SpaceWithNewline)
        }
        SpaceState::Supportive { text: prev_text } => {
            (SpaceAction::Keep, SpaceState::Space { prev_text })
        }
        // Spaces
        SpaceState::Space { prev_text: Some(text), .. }
            if had_nl && text.chars().next_back().is_some_and(is_space_discarding) =>
        {
            (SpaceAction::DiscardAndSkip, SpaceState::Destructive)
        }
        SpaceState::Space { .. } if had_nl => {
            (SpaceAction::Skip, SpaceState::SpaceWithNewline)
        }
        space @ (SpaceState::Space { .. } | SpaceState::SpaceWithNewline) => {
            (SpaceAction::Skip, space)
        }
    }
}

/// Whether a character is part of the space-discarding set for Typst. These
/// characters discard adjacent spaces caused by newlines and allow Chinese and
/// Japanese text to be broken across lines in markup without producing spaces.
///
/// Currently this checks if the character is in either the Chinese or Japanese
/// scripts, or it is Common script (mainly punctuation) and has a defined East
/// Asian Width property of H/F/W and is not an Emoji.
pub(crate) fn is_space_discarding(c: char) -> bool {
    // TODO: Load ICU sets/maps from typst-assets or use data from a different
    // crate altogether. I assume there are still more changes to make, so
    // leaving as-is for now.
    const SCRIPT_DATA: CodePointMapDataBorrowed<'static, Script> =
        icu_properties::maps::script();
    const EAW_DATA: CodePointMapDataBorrowed<'static, EastAsianWidth> =
        icu_properties::maps::east_asian_width();
    const EMOJI_DATA: CodePointSetDataBorrowed<'static> = icu_properties::sets::emoji();

    match SCRIPT_DATA.get(c) {
        Script::Han | Script::Hiragana | Script::Katakana => true,
        Script::Common => {
            matches!(
                EAW_DATA.get(c),
                EastAsianWidth::Halfwidth
                    | EastAsianWidth::Fullwidth
                    | EastAsianWidth::Wide
            ) && !EMOJI_DATA.contains(c)
        }
        _ => false,
    }
}

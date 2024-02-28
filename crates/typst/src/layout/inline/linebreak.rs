use icu_properties::maps::CodePointMapData;
use icu_properties::LineBreak;
use icu_provider::AsDeserializingBufferProvider;
use icu_provider_adapters::fork::ForkByKeyProvider;
use icu_provider_blob::BlobDataProvider;
use icu_segmenter::LineSegmenter;
use once_cell::sync::Lazy;

use super::Preparation;
use crate::syntax::link_prefix;
use crate::text::{Lang, TextElem};

/// The general line break segmenter.
static SEGMENTER: Lazy<LineSegmenter> = Lazy::new(|| {
    let provider =
        BlobDataProvider::try_new_from_static_blob(typst_assets::icu::ICU).unwrap();
    LineSegmenter::try_new_lstm_with_buffer_provider(&provider).unwrap()
});

/// The line break segmenter for Chinese/Japanese text.
static CJ_SEGMENTER: Lazy<LineSegmenter> = Lazy::new(|| {
    let provider =
        BlobDataProvider::try_new_from_static_blob(typst_assets::icu::ICU).unwrap();
    let cj_blob =
        BlobDataProvider::try_new_from_static_blob(typst_assets::icu::ICU_CJ_SEGMENT)
            .unwrap();
    let cj_provider = ForkByKeyProvider::new(cj_blob, provider);
    LineSegmenter::try_new_lstm_with_buffer_provider(&cj_provider).unwrap()
});

/// The Unicode line break properties for each code point.
static LINEBREAK_DATA: Lazy<CodePointMapData<LineBreak>> = Lazy::new(|| {
    let provider =
        BlobDataProvider::try_new_from_static_blob(typst_assets::icu::ICU).unwrap();
    let deser_provider = provider.as_deserializing();
    icu_properties::maps::load_line_break(&deser_provider).unwrap()
});

/// A line break opportunity.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum Breakpoint {
    /// Just a normal opportunity (e.g. after a space).
    Normal,
    /// A mandatory breakpoint (after '\n' or at the end of the text).
    Mandatory,
    /// An opportunity for hyphenating.
    Hyphen,
}

/// Calls `f` for all possible points in the text where lines can broken.
///
/// Yields for each breakpoint the text index, whether the break is mandatory
/// (after `\n`) and whether a hyphen is required (when breaking inside of a
/// word).
///
/// This is an internal instead of an external iterator because it makes the
/// code much simpler and the consumers of this function don't need the
/// composability and flexibility of external iteration anyway.
pub(super) fn breakpoints<'a>(
    p: &'a Preparation<'a>,
    mut f: impl FnMut(usize, Breakpoint),
) {
    let text = p.bidi.text;
    let hyphenate = p.hyphenate != Some(false);
    let lb = LINEBREAK_DATA.as_borrowed();
    let segmenter = match p.lang {
        Some(Lang::CHINESE | Lang::JAPANESE) => &CJ_SEGMENTER,
        _ => &SEGMENTER,
    };

    let mut last = 0;
    let mut iter = segmenter.segment_str(text).peekable();

    loop {
        // Special case for links. UAX #14 doesn't handle them well.
        let (head, tail) = text.split_at(last);
        if head.ends_with("://") || tail.starts_with("www.") {
            let (link, _) = link_prefix(tail);
            let end = last + link.len();
            linebreak_link(link, |i| f(last + i, Breakpoint::Normal));
            while iter.peek().is_some_and(|&p| p < end) {
                iter.next();
            }
        }

        // Get the UAX #14 linebreak opportunities.
        let Some(point) = iter.next() else { break };

        // Skip breakpoint if there is no char before it. icu4x generates one
        // at offset 0, but we don't want it.
        let Some(c) = text[..point].chars().next_back() else { continue };

        // Find out whether the last break was mandatory by checking against
        // rules LB4 and LB5, special-casing the end of text according to LB3.
        // See also: https://docs.rs/icu_segmenter/latest/icu_segmenter/struct.LineSegmenter.html
        let breakpoint = if point == text.len() {
            Breakpoint::Mandatory
        } else {
            match lb.get(c) {
                // Fix for: https://github.com/unicode-org/icu4x/issues/4146
                LineBreak::Glue | LineBreak::WordJoiner | LineBreak::ZWJ => continue,
                LineBreak::MandatoryBreak
                | LineBreak::CarriageReturn
                | LineBreak::LineFeed
                | LineBreak::NextLine => Breakpoint::Mandatory,
                _ => Breakpoint::Normal,
            }
        };

        // Hyphenate between the last and current breakpoint.
        'hyphenate: {
            if !hyphenate {
                break 'hyphenate;
            }

            // Extract a hyphenatable "word".
            let word = &text[last..point].trim_end_matches(|c: char| !c.is_alphabetic());
            if word.is_empty() {
                break 'hyphenate;
            }

            let end = last + word.len();
            let mut offset = last;

            // Determine the language to hyphenate this word in.
            let Some(lang) = lang_at(p, last) else { break 'hyphenate };

            for syllable in hypher::hyphenate(word, lang) {
                // Don't hyphenate after the final syllable.
                offset += syllable.len();
                if offset == end {
                    continue;
                }

                // Filter out hyphenation opportunities where hyphenation was
                // actually disabled.
                if !hyphenate_at(p, offset) {
                    continue;
                }

                // Filter out forbidden hyphenation opportunities.
                if matches!(
                    syllable.chars().next_back().map(|c| lb.get(c)),
                    Some(LineBreak::Glue | LineBreak::WordJoiner | LineBreak::ZWJ)
                ) {
                    continue;
                }

                // Call `f` for the word-internal hyphenation opportunity.
                f(offset, Breakpoint::Hyphen);
            }
        }

        // Call `f` for the UAX #14 break opportunity.
        f(point, breakpoint);

        last = point;
    }
}

/// Produce linebreak opportunities for a link.
fn linebreak_link(link: &str, mut f: impl FnMut(usize)) {
    #[derive(PartialEq)]
    enum Class {
        Alphabetic,
        Digit,
        Open,
        Other,
    }

    impl Class {
        fn of(c: char) -> Self {
            if c.is_alphabetic() {
                Class::Alphabetic
            } else if c.is_numeric() {
                Class::Digit
            } else if matches!(c, '(' | '[') {
                Class::Open
            } else {
                Class::Other
            }
        }
    }

    let mut offset = 0;
    let mut prev = Class::Other;

    for (end, c) in link.char_indices() {
        let class = Class::of(c);

        // Emit opportunities when going from
        // - other -> other
        // - alphabetic -> numeric
        // - numeric -> alphabetic
        // Never before after opening delimiters.
        if end > 0
            && prev != Class::Open
            && if class == Class::Other { prev == Class::Other } else { class != prev }
        {
            let piece = &link[offset..end];
            if piece.len() < 16 {
                // For bearably long segments, emit them as one.
                offset = end;
                f(offset);
            } else {
                // If it gets very long (e.g. a hash in the URL), just allow a
                // break at every char.
                for c in piece.chars() {
                    offset += c.len_utf8();
                    f(offset);
                }
            }
        }

        prev = class;
    }
}

/// Whether hyphenation is enabled at the given offset.
fn hyphenate_at(p: &Preparation, offset: usize) -> bool {
    p.hyphenate
        .or_else(|| {
            let shaped = p.find(offset)?.text()?;
            Some(TextElem::hyphenate_in(shaped.styles))
        })
        .unwrap_or(false)
}

/// The text language at the given offset.
fn lang_at(p: &Preparation, offset: usize) -> Option<hypher::Lang> {
    let lang = p.lang.or_else(|| {
        let shaped = p.find(offset)?.text()?;
        Some(TextElem::lang_in(shaped.styles))
    })?;

    let bytes = lang.as_str().as_bytes().try_into().ok()?;
    hypher::Lang::from_iso(bytes)
}

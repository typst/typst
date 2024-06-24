use icu_properties::maps::CodePointMapData;
use icu_properties::LineBreak;
use icu_provider::AsDeserializingBufferProvider;
use icu_provider_adapters::fork::ForkByKeyProvider;
use icu_provider_blob::BlobDataProvider;
use icu_segmenter::LineSegmenter;
use once_cell::sync::Lazy;

use super::*;
use crate::engine::Engine;
use crate::layout::Abs;
use crate::model::Linebreaks;
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
pub enum Breakpoint {
    /// Just a normal opportunity (e.g. after a space).
    Normal,
    /// A mandatory breakpoint (after '\n' or at the end of the text).
    Mandatory,
    /// An opportunity for hyphenating.
    Hyphen,
}

/// Breaks the paragraph into lines.
pub fn linebreak<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    width: Abs,
) -> Vec<Line<'a>> {
    let linebreaks = p.linebreaks.unwrap_or_else(|| {
        if p.justify {
            Linebreaks::Optimized
        } else {
            Linebreaks::Simple
        }
    });

    match linebreaks {
        Linebreaks::Simple => linebreak_simple(engine, p, width),
        Linebreaks::Optimized => linebreak_optimized(engine, p, width),
    }
}

/// Performs line breaking in simple first-fit style. This means that we build
/// lines greedily, always taking the longest possible line. This may lead to
/// very unbalanced line, but is fast and simple.
#[typst_macros::time]
fn linebreak_simple<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    width: Abs,
) -> Vec<Line<'a>> {
    let mut lines = Vec::with_capacity(16);
    let mut start = 0;
    let mut last = None;

    breakpoints(p, |end, breakpoint| {
        let prepend_hyphen = lines.last().map(should_repeat_hyphen).unwrap_or(false);

        // Compute the line and its size.
        let mut attempt = line(engine, p, start..end, breakpoint, prepend_hyphen);

        // If the line doesn't fit anymore, we push the last fitting attempt
        // into the stack and rebuild the line from the attempt's end. The
        // resulting line cannot be broken up further.
        if !width.fits(attempt.width) {
            if let Some((last_attempt, last_end)) = last.take() {
                lines.push(last_attempt);
                start = last_end;
                attempt = line(engine, p, start..end, breakpoint, prepend_hyphen);
            }
        }

        // Finish the current line if there is a mandatory line break (i.e. due
        // to "\n") or if the line doesn't fit horizontally already since then
        // no shorter line will be possible.
        if breakpoint == Breakpoint::Mandatory || !width.fits(attempt.width) {
            lines.push(attempt);
            start = end;
            last = None;
        } else {
            last = Some((attempt, end));
        }
    });

    if let Some((line, _)) = last {
        lines.push(line);
    }

    lines
}

/// Performs line breaking in optimized Knuth-Plass style. Here, we use more
/// context to determine the line breaks than in the simple first-fit style. For
/// example, we might choose to cut a line short even though there is still a
/// bit of space to improve the fit of one of the following lines. The
/// Knuth-Plass algorithm is based on the idea of "cost". A line which has a
/// very tight or very loose fit has a higher cost than one that is just right.
/// Ending a line with a hyphen incurs extra cost and endings two successive
/// lines with hyphens even more.
///
/// To find the layout with the minimal total cost the algorithm uses dynamic
/// programming: For each possible breakpoint it determines the optimal
/// paragraph layout _up to that point_. It walks over all possible start points
/// for a line ending at that point and finds the one for which the cost of the
/// line plus the cost of the optimal paragraph up to the start point (already
/// computed and stored in dynamic programming table) is minimal. The final
/// result is simply the layout determined for the last breakpoint at the end of
/// text.
#[typst_macros::time]
fn linebreak_optimized<'a>(
    engine: &Engine,
    p: &'a Preparation<'a>,
    width: Abs,
) -> Vec<Line<'a>> {
    /// The cost of a line or paragraph layout.
    type Cost = f64;

    /// An entry in the dynamic programming table.
    struct Entry<'a> {
        pred: usize,
        total: Cost,
        line: Line<'a>,
    }

    // Cost parameters.
    const DEFAULT_HYPH_COST: Cost = 0.5;
    const DEFAULT_RUNT_COST: Cost = 0.5;
    const CONSECUTIVE_DASH_COST: Cost = 0.3;
    const MAX_COST: Cost = 1_000_000.0;
    const MIN_RATIO: f64 = -1.0;

    let hyph_cost = DEFAULT_HYPH_COST * p.costs.hyphenation().get();
    let runt_cost = DEFAULT_RUNT_COST * p.costs.runt().get();

    // Dynamic programming table.
    let mut active = 0;
    let mut table = vec![Entry {
        pred: 0,
        total: 0.0,
        line: line(engine, p, 0..0, Breakpoint::Mandatory, false),
    }];

    let em = p.size;
    let mut lines = Vec::with_capacity(16);
    breakpoints(p, |end, breakpoint| {
        let k = table.len();
        let is_end = end == p.bidi.text.len();
        let mut best: Option<Entry> = None;

        // Find the optimal predecessor.
        for (i, pred) in table.iter().enumerate().skip(active) {
            // Layout the line.
            let start = pred.line.end;
            let prepend_hyphen = should_repeat_hyphen(&pred.line);

            let attempt = line(engine, p, start..end, breakpoint, prepend_hyphen);

            // Determine how much the line's spaces would need to be stretched
            // to make it the desired width.
            let delta = width - attempt.width;
            // Determine how much stretch are permitted.
            let adjust = if delta >= Abs::zero() {
                attempt.stretchability()
            } else {
                attempt.shrinkability()
            };
            // Ideally, the ratio should between -1.0 and 1.0, but sometimes a
            // value above 1.0 is possible, in which case the line is underfull.
            let mut ratio = delta / adjust;
            if ratio.is_nan() {
                // The line is not stretchable, but it just fits. This often
                // happens with monospace fonts and CJK texts.
                ratio = 0.0;
            }
            if ratio > 1.0 {
                // We should stretch the line above its stretchability. Now
                // calculate the extra amount. Also, don't divide by zero.
                let extra_stretch =
                    (delta - adjust) / attempt.justifiables().max(1) as f64;
                // Normalize the amount by half Em size.
                ratio = 1.0 + extra_stretch / (em / 2.0);
            }

            // Determine the cost of the line.
            let min_ratio = if p.justify { MIN_RATIO } else { 0.0 };
            let mut cost = if ratio < min_ratio {
                // The line is overfull. This is the case if
                // - justification is on, but we'd need to shrink too much
                // - justification is off and the line just doesn't fit
                //
                // If this is the earliest breakpoint in the active set
                // (active == i), remove it from the active set. If there is an
                // earlier one (active < i), then the logically shorter line was
                // in fact longer (can happen with negative spacing) and we
                // can't trim the active set just yet.
                if active == i {
                    active += 1;
                }
                MAX_COST
            } else if breakpoint == Breakpoint::Mandatory || is_end {
                // This is a mandatory break and the line is not overfull, so
                // all breakpoints before this one become inactive since no line
                // can span above the mandatory break.
                active = k;
                // - If ratio > 0, we need to stretch the line only when justify
                //   is needed.
                // - If ratio < 0, we always need to shrink the line.
                if (ratio > 0.0 && attempt.justify) || ratio < 0.0 {
                    ratio.powi(3).abs()
                } else {
                    0.0
                }
            } else {
                // Normal line with cost of |ratio^3|.
                ratio.powi(3).abs()
            };

            // Penalize runts.
            if k == i + 1 && is_end {
                cost += runt_cost;
            }

            // Penalize hyphens.
            if breakpoint == Breakpoint::Hyphen {
                cost += hyph_cost;
            }

            // In Knuth paper, cost = (1 + 100|r|^3 + p)^2 + a,
            // where r is the ratio, p=50 is the penalty, and a=3000 is
            // consecutive the penalty. We divide the whole formula by 10,
            // resulting (0.01 + |r|^3 + p)^2 + a, where p=0.5 and a=0.3
            cost = (0.01 + cost).powi(2);

            // Penalize two consecutive dashes (not necessarily hyphens) extra.
            if attempt.dash.is_some() && pred.line.dash.is_some() {
                cost += CONSECUTIVE_DASH_COST;
            }

            // The total cost of this line and its chain of predecessors.
            let total = pred.total + cost;

            // If this attempt is better than what we had before, take it!
            if best.as_ref().map_or(true, |best| best.total >= total) {
                best = Some(Entry { pred: i, total, line: attempt });
            }
        }

        table.push(best.unwrap());
    });

    // Retrace the best path.
    let mut idx = table.len() - 1;
    while idx != 0 {
        table.truncate(idx + 1);
        let entry = table.pop().unwrap();
        lines.push(entry.line);
        idx = entry.pred;
    }

    lines.reverse();
    lines
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
fn breakpoints<'a>(p: &'a Preparation<'a>, mut f: impl FnMut(usize, Breakpoint)) {
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

/// Whether the hyphen should repeat at the start of the next line.
fn should_repeat_hyphen(pred_line: &Line) -> bool {
    // If the predecessor line does not end with a Dash::HardHyphen, we shall
    // not place a hyphen at the start of the next line.
    if pred_line.dash != Some(Dash::HardHyphen) {
        return false;
    }

    // If there's a trimmed out space, we needn't repeat the hyphen. That's the
    // case of a text like "...kebab é a -melhor- comida que existe", where the
    // hyphens are a kind of emphasis marker.
    if pred_line.trimmed.end != pred_line.end {
        return false;
    }

    // The hyphen should repeat only in the languages that require that feature.
    // For more information see the discussion at https://github.com/typst/typst/issues/3235
    let Some(Item::Text(shape)) = pred_line.last.as_ref() else { return false };

    match shape.lang {
        // - Lower Sorbian: see https://dolnoserbski.de/ortografija/psawidla/K3
        // - Czech: see https://prirucka.ujc.cas.cz/?id=164
        // - Croatian: see http://pravopis.hr/pravilo/spojnica/68/
        // - Polish: see https://www.ortograf.pl/zasady-pisowni/lacznik-zasady-pisowni
        // - Portuguese: see https://www2.senado.leg.br/bdsf/bitstream/handle/id/508145/000997415.pdf (Base XX)
        // - Slovak: see https://www.zones.sk/studentske-prace/gramatika/10620-pravopis-rozdelovanie-slov/
        Lang::LOWER_SORBIAN
        | Lang::CZECH
        | Lang::CROATIAN
        | Lang::POLISH
        | Lang::PORTUGUESE
        | Lang::SLOVAK => true,
        // In Spanish the hyphen is required only if the word next to hyphen is
        // not capitalized. Otherwise, the hyphen must not be repeated.
        //
        // See § 4.1.1.1.2.e on the "Ortografía de la lengua española"
        // https://www.rae.es/ortografía/como-signo-de-división-de-palabras-a-final-de-línea
        Lang::SPANISH => pred_line.bidi.text[pred_line.end..]
            .chars()
            .next()
            .map(|c| !c.is_uppercase())
            .unwrap_or(false),
        _ => false,
    }
}

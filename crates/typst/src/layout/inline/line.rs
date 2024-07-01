use unicode_bidi::BidiInfo;

use super::*;
use crate::engine::Engine;
use crate::layout::{Abs, Em, Fr, Frame, FrameItem, Point};
use crate::text::{Lang, TextElem};
use crate::utils::Numeric;

/// A layouted line, consisting of a sequence of layouted paragraph items that
/// are mostly borrowed from the preparation phase. This type enables you to
/// measure the size of a line in a range before committing to building the
/// line's frame.
///
/// At most two paragraph items must be created individually for this line: The
/// first and last one since they may be broken apart by the start or end of the
/// line, respectively. But even those can partially reuse previous results when
/// the break index is safe-to-break per rustybuzz.
pub struct Line<'a> {
    /// Bidi information about the paragraph.
    pub bidi: &'a BidiInfo<'a>,
    /// The trimmed range the line spans in the paragraph.
    pub trimmed: Range,
    /// The untrimmed end where the line ends.
    pub end: usize,
    /// A reshaped text item if the line sliced up a text item at the start.
    pub first: Option<Item<'a>>,
    /// Inner items which don't need to be reprocessed.
    pub inner: &'a [Item<'a>],
    /// A reshaped text item if the line sliced up a text item at the end. If
    /// there is only one text item, this takes precedence over `first`.
    pub last: Option<Item<'a>>,
    /// The width of the line.
    pub width: Abs,
    /// Whether the line should be justified.
    pub justify: bool,
    /// Whether the line ends with a hyphen or dash, either naturally or through
    /// hyphenation.
    pub dash: Option<Dash>,
}

impl<'a> Line<'a> {
    /// Iterate over the line's items.
    pub fn items(&self) -> impl Iterator<Item = &Item<'a>> {
        self.first.iter().chain(self.inner).chain(&self.last)
    }

    /// Return items that intersect the given `text_range`.
    pub fn slice(&self, text_range: Range) -> impl Iterator<Item = &Item<'a>> {
        let mut cursor = self.trimmed.start;
        let mut start = 0;
        let mut end = 0;

        for (i, item) in self.items().enumerate() {
            if cursor <= text_range.start {
                start = i;
            }

            let len = item.textual_len();
            if cursor < text_range.end || cursor + len <= text_range.end {
                end = i + 1;
            } else {
                break;
            }

            cursor += len;
        }

        self.items().skip(start).take(end - start)
    }

    /// How many glyphs are in the text where we can insert additional
    /// space when encountering underfull lines.
    pub fn justifiables(&self) -> usize {
        let mut count = 0;
        for shaped in self.items().filter_map(Item::text) {
            count += shaped.justifiables();
        }
        // CJK character at line end should not be adjusted.
        if self
            .items()
            .last()
            .and_then(Item::text)
            .map(|s| s.cjk_justifiable_at_last())
            .unwrap_or(false)
        {
            count -= 1;
        }

        count
    }

    /// How much can the line stretch
    pub fn stretchability(&self) -> Abs {
        self.items().filter_map(Item::text).map(|s| s.stretchability()).sum()
    }

    /// How much can the line shrink
    pub fn shrinkability(&self) -> Abs {
        self.items().filter_map(Item::text).map(|s| s.shrinkability()).sum()
    }

    /// Whether the line has items with negative width.
    pub fn has_negative_width_items(&self) -> bool {
        self.items().any(|item| match item {
            Item::Absolute(amount, _) => *amount < Abs::zero(),
            Item::Frame(frame, _) => frame.width() < Abs::zero(),
            _ => false,
        })
    }

    /// The sum of fractions in the line.
    pub fn fr(&self) -> Fr {
        self.items()
            .filter_map(|item| match item {
                Item::Fractional(fr, _) => Some(*fr),
                _ => None,
            })
            .sum()
    }
}

/// A dash at the end of a line.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Dash {
    /// A hyphen added to break a word.
    SoftHyphen,
    /// Regular hyphen, present in a compound word, e.g. beija-flor.
    HardHyphen,
    /// An em dash.
    Long,
    /// An en dash.
    Short,
}

/// Create a line which spans the given range.
pub fn line<'a>(
    engine: &Engine,
    p: &'a Preparation,
    mut range: Range,
    breakpoint: Breakpoint,
    pred: Option<&Line>,
) -> Line<'a> {
    let end = range.end;
    let mut justify =
        p.justify && end < p.bidi.text.len() && breakpoint != Breakpoint::Mandatory;

    if range.is_empty() {
        return Line {
            bidi: &p.bidi,
            end,
            trimmed: range,
            first: None,
            inner: &[],
            last: None,
            width: Abs::zero(),
            justify,
            dash: None,
        };
    }

    let prepend_hyphen = pred.map_or(false, should_insert_hyphen);

    // Slice out the relevant items.
    let (mut expanded, mut inner) = p.slice(range.clone());
    let mut width = Abs::zero();

    // Weak space (`Absolute(_, true)`) is removed at the end of the line
    while let Some((Item::Absolute(_, true), before)) = inner.split_last() {
        inner = before;
        range.end -= 1;
        expanded.end -= 1;
    }
    // Weak space (`Absolute(_, true)`) is removed at the beginning of the line
    while let Some((Item::Absolute(_, true), after)) = inner.split_first() {
        inner = after;
        range.start += 1;
        expanded.end += 1;
    }

    // Reshape the last item if it's split in half or hyphenated.
    let mut last = None;
    let mut dash = None;
    if let Some((Item::Text(shaped), before)) = inner.split_last() {
        // Compute the range we want to shape, trimming whitespace at the
        // end of the line.
        let base = expanded.end - shaped.text.len();
        let start = range.start.max(base);
        let text = &p.bidi.text[start..range.end];
        // U+200B ZERO WIDTH SPACE is used to provide a line break opportunity,
        // we want to trim it too.
        let trimmed = text.trim_end().trim_end_matches('\u{200B}');
        range.end = start + trimmed.len();

        // Deal with hyphens, dashes and justification.
        let shy = trimmed.ends_with('\u{ad}');
        let hyphen = breakpoint == Breakpoint::Hyphen;
        dash = if hyphen || shy {
            Some(Dash::SoftHyphen)
        } else if trimmed.ends_with('-') {
            Some(Dash::HardHyphen)
        } else if trimmed.ends_with('–') {
            Some(Dash::Short)
        } else if trimmed.ends_with('—') {
            Some(Dash::Long)
        } else {
            None
        };
        justify |= text.ends_with('\u{2028}');

        // Deal with CJK punctuation at line ends.
        let gb_style = cjk_punct_style(shaped.lang, shaped.region);
        let maybe_adjust_last_glyph = trimmed.ends_with(END_PUNCT_PAT)
            || (p.cjk_latin_spacing && trimmed.ends_with(is_of_cj_script));

        // Usually, we don't want to shape an empty string because:
        // - We don't want the height of trimmed whitespace in a different font
        //   to be considered for the line height.
        // - Even if it's in the same font, its unnecessary.
        //
        // There is one exception though. When the whole line is empty, we need
        // the shaped empty string to make the line the appropriate height. That
        // is the case exactly if the string is empty and there are no other
        // items in the line.
        if hyphen
            || start + shaped.text.len() > range.end
            || maybe_adjust_last_glyph
            || prepend_hyphen
        {
            if hyphen || start < range.end || before.is_empty() {
                let mut reshaped = shaped.reshape(engine, &p.spans, start..range.end);
                if hyphen || shy {
                    reshaped.push_hyphen(engine, p.fallback);
                }

                if let Some(last_glyph) = reshaped.glyphs.last() {
                    if last_glyph.is_cjk_left_aligned_punctuation(gb_style) {
                        // If the last glyph is a CJK punctuation, we want to
                        // shrink it. See Requirements for Chinese Text Layout,
                        // Section 3.1.6.3 Compression of punctuation marks at
                        // line start or line end
                        let shrink_amount = last_glyph.shrinkability().1;
                        let punct = reshaped.glyphs.to_mut().last_mut().unwrap();
                        punct.shrink_right(shrink_amount);
                        reshaped.width -= shrink_amount.at(reshaped.size);
                    } else if p.cjk_latin_spacing
                        && last_glyph.is_cj_script()
                        && (last_glyph.x_advance - last_glyph.x_offset) > Em::one()
                    {
                        // If the last glyph is a CJK character adjusted by
                        // [`add_cjk_latin_spacing`], restore the original
                        // width.
                        let shrink_amount =
                            last_glyph.x_advance - last_glyph.x_offset - Em::one();
                        let glyph = reshaped.glyphs.to_mut().last_mut().unwrap();
                        glyph.x_advance -= shrink_amount;
                        glyph.adjustability.shrinkability.1 = Em::zero();
                        reshaped.width -= shrink_amount.at(reshaped.size);
                    }
                }

                width += reshaped.width;
                last = Some(Item::Text(reshaped));
            }

            inner = before;
        }
    }

    // Deal with CJ characters at line starts.
    let text = &p.bidi.text[range.start..end];
    let maybe_adjust_first_glyph = text.starts_with(BEGIN_PUNCT_PAT)
        || (p.cjk_latin_spacing && text.starts_with(is_of_cj_script));

    // Reshape the start item if it's split in half.
    let mut first = None;
    if let Some((Item::Text(shaped), after)) = inner.split_first() {
        // Compute the range we want to shape.
        let base = expanded.start;
        let end = range.end.min(base + shaped.text.len());

        // Reshape if necessary.
        if range.start + shaped.text.len() > end
            || maybe_adjust_first_glyph
            || prepend_hyphen
        {
            // If the range is empty, we don't want to push an empty text item.
            if range.start < end {
                let reshaped = shaped.reshape(engine, &p.spans, range.start..end);
                width += reshaped.width;
                first = Some(Item::Text(reshaped));
            }

            inner = after;
        }
    }

    if prepend_hyphen {
        let reshaped = first.as_mut().or(last.as_mut()).and_then(Item::text_mut);
        if let Some(reshaped) = reshaped {
            let width_before = reshaped.width;
            reshaped.prepend_hyphen(engine, p.fallback);
            width += reshaped.width - width_before;
        }
    }

    if maybe_adjust_first_glyph {
        let reshaped = first.as_mut().or(last.as_mut()).and_then(Item::text_mut);
        if let Some(reshaped) = reshaped {
            if let Some(first_glyph) = reshaped.glyphs.first() {
                if first_glyph.is_cjk_right_aligned_punctuation() {
                    // If the first glyph is a CJK punctuation, we want to
                    // shrink it.
                    let shrink_amount = first_glyph.shrinkability().0;
                    let glyph = reshaped.glyphs.to_mut().first_mut().unwrap();
                    glyph.shrink_left(shrink_amount);
                    let amount_abs = shrink_amount.at(reshaped.size);
                    reshaped.width -= amount_abs;
                    width -= amount_abs;
                } else if p.cjk_latin_spacing
                    && first_glyph.is_cj_script()
                    && first_glyph.x_offset > Em::zero()
                {
                    // If the first glyph is a CJK character adjusted by
                    // [`add_cjk_latin_spacing`], restore the original width.
                    let shrink_amount = first_glyph.x_offset;
                    let glyph = reshaped.glyphs.to_mut().first_mut().unwrap();
                    glyph.x_advance -= shrink_amount;
                    glyph.x_offset = Em::zero();
                    glyph.adjustability.shrinkability.0 = Em::zero();
                    let amount_abs = shrink_amount.at(reshaped.size);
                    reshaped.width -= amount_abs;
                    width -= amount_abs;
                }
            }
        }
    }

    // Measure the inner items.
    for item in inner {
        width += item.width();
    }

    Line {
        bidi: &p.bidi,
        trimmed: range,
        end,
        first,
        inner,
        last,
        width,
        justify,
        dash,
    }
}

/// Commit to a line and build its frame.
pub fn commit(
    engine: &mut Engine,
    p: &Preparation,
    line: &Line,
    width: Abs,
    full: Abs,
    shrink: bool,
) -> SourceResult<Frame> {
    let mut remaining = width - line.width - p.hang;
    let mut offset = Abs::zero();

    // Reorder the line from logical to visual order.
    let (reordered, starts_rtl) = reorder(line);
    if !starts_rtl {
        offset += p.hang;
    }

    // Handle hanging punctuation to the left.
    if let Some(Item::Text(text)) = reordered.first() {
        if let Some(glyph) = text.glyphs.first() {
            if !text.dir.is_positive()
                && TextElem::overhang_in(text.styles)
                && (reordered.len() > 1 || text.glyphs.len() > 1)
            {
                let amount = overhang(glyph.c) * glyph.x_advance.at(text.size);
                offset -= amount;
                remaining += amount;
            }
        }
    }

    // Handle hanging punctuation to the right.
    if let Some(Item::Text(text)) = reordered.last() {
        if let Some(glyph) = text.glyphs.last() {
            if text.dir.is_positive()
                && TextElem::overhang_in(text.styles)
                && (reordered.len() > 1 || text.glyphs.len() > 1)
            {
                let amount = overhang(glyph.c) * glyph.x_advance.at(text.size);
                remaining += amount;
            }
        }
    }

    // Determine how much additional space is needed. The justification_ratio is
    // for the first step justification, extra_justification is for the last
    // step. For more info on multi-step justification, see Procedures for
    // Inter- Character Space Expansion in W3C document Chinese Layout
    // Requirements.
    let fr = line.fr();
    let mut justification_ratio = 0.0;
    let mut extra_justification = Abs::zero();

    let shrinkability = line.shrinkability();
    let stretch = line.stretchability();
    if remaining < Abs::zero() && shrinkability > Abs::zero() && shrink {
        // Attempt to reduce the length of the line, using shrinkability.
        justification_ratio = (remaining / shrinkability).max(-1.0);
        remaining = (remaining + shrinkability).min(Abs::zero());
    } else if line.justify && fr.is_zero() {
        // Attempt to increase the length of the line, using stretchability.
        if stretch > Abs::zero() {
            justification_ratio = (remaining / stretch).min(1.0);
            remaining = (remaining - stretch).max(Abs::zero());
        }

        let justifiables = line.justifiables();
        if justifiables > 0 && remaining > Abs::zero() {
            // Underfull line, distribute the extra space.
            extra_justification = remaining / justifiables as f64;
            remaining = Abs::zero();
        }
    }

    let mut top = Abs::zero();
    let mut bottom = Abs::zero();

    // Build the frames and determine the height and baseline.
    let mut frames = vec![];
    for item in reordered {
        let mut push = |offset: &mut Abs, frame: Frame| {
            let width = frame.width();
            top.set_max(frame.baseline());
            bottom.set_max(frame.size().y - frame.baseline());
            frames.push((*offset, frame));
            *offset += width;
        };

        match item {
            Item::Absolute(v, _) => {
                offset += *v;
            }
            Item::Fractional(v, elem) => {
                let amount = v.share(fr, remaining);
                if let Some((elem, loc, styles)) = elem {
                    let region = Size::new(amount, full);
                    let mut frame =
                        elem.layout(engine, loc.relayout(), *styles, region)?;
                    frame.post_process(*styles);
                    frame.translate(Point::with_y(TextElem::baseline_in(*styles)));
                    push(&mut offset, frame);
                } else {
                    offset += amount;
                }
            }
            Item::Text(shaped) => {
                let mut frame =
                    shaped.build(engine, justification_ratio, extra_justification);
                frame.post_process(shaped.styles);
                push(&mut offset, frame);
            }
            Item::Frame(frame, styles) => {
                let mut frame = frame.clone();
                frame.post_process(*styles);
                frame.translate(Point::with_y(TextElem::baseline_in(*styles)));
                push(&mut offset, frame);
            }
            Item::Tag(tag) => {
                let mut frame = Frame::soft(Size::zero());
                frame.push(Point::zero(), FrameItem::Tag((*tag).clone()));
                frames.push((offset, frame));
            }
            Item::Skip(_) => {}
        }
    }

    // Remaining space is distributed now.
    if !fr.is_zero() {
        remaining = Abs::zero();
    }

    let size = Size::new(width, top + bottom);
    let mut output = Frame::soft(size);
    output.set_baseline(top);

    // Construct the line's frame.
    for (offset, frame) in frames {
        let x = offset + p.align.position(remaining);
        let y = top - frame.baseline();
        output.push_frame(Point::new(x, y), frame);
    }

    Ok(output)
}

/// Return a line's items in visual order.
fn reorder<'a>(line: &'a Line<'a>) -> (Vec<&Item<'a>>, bool) {
    let mut reordered = vec![];

    // The bidi crate doesn't like empty lines.
    if line.trimmed.is_empty() {
        return (line.slice(line.trimmed.clone()).collect(), false);
    }

    // Find the paragraph that contains the line.
    let para = line
        .bidi
        .paragraphs
        .iter()
        .find(|para| para.range.contains(&line.trimmed.start))
        .unwrap();

    // Compute the reordered ranges in visual order (left to right).
    let (levels, runs) = line.bidi.visual_runs(para, line.trimmed.clone());
    let starts_rtl = levels.first().is_some_and(|level| level.is_rtl());

    // Collect the reordered items.
    for run in runs {
        // Skip reset L1 runs because handling them would require reshaping
        // again in some cases.
        if line.bidi.levels[run.start] != levels[run.start] {
            continue;
        }

        let prev = reordered.len();
        reordered.extend(line.slice(run.clone()));

        if levels[run.start].is_rtl() {
            reordered[prev..].reverse();
        }
    }

    (reordered, starts_rtl)
}

/// Whether a hyphen should be inserted at the start of the next line.
fn should_insert_hyphen(pred_line: &Line) -> bool {
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

/// How much a character should hang into the end margin.
///
/// For more discussion, see:
/// <https://recoveringphysicist.com/21/>
fn overhang(c: char) -> f64 {
    match c {
        // Dashes.
        '–' | '—' => 0.2,
        '-' => 0.55,

        // Punctuation.
        '.' | ',' => 0.8,
        ':' | ';' => 0.3,

        // Arabic
        '\u{60C}' | '\u{6D4}' => 0.4,

        _ => 0.0,
    }
}

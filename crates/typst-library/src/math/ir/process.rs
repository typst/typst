use std::iter;
use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;
use unicode_math_class::MathClass;

use super::item::{MathItem, RawMathItem, RowMeta};
use super::multiline::{AlignedRow, split_at_align};
use crate::foundations::{Smart, StyleChain};
use typst_syntax::Span;
use crate::math::{MEDIUM, MathSize, THICK, THIN};

/// The result of processing items for grouping.
pub(crate) enum GroupResult<'a> {
    /// Linebreaks weren't present and alignment points were pruned giving plain
    /// items.
    Flat(Vec<MathItem<'a>>),
    /// Linebreaks were present, and items are split into padded rows and
    /// alignment columns.
    Multiline(Vec<AlignedRow<'a>>, Vec<RowMeta<'a>>),
}

/// Processes raw items for grouping.
///
/// The `closing` parameter indicates whether a closing delimiter follows the
/// items. The `pad` parameter indicates whether, when linebreaks are present,
/// the resulting rows should be padded to have the same length.
pub(crate) fn process_group<'a, I>(
    items: I,
    styles: StyleChain<'a>,
    closing: bool,
    pad: bool,
) -> GroupResult<'a>
where
    I: IntoIterator<Item = RawMathItem<'a>>,
    I::IntoIter: ExactSizeIterator,
{
    let preprocessed = preprocess(items, closing, false);
    if preprocessed.linebreaks > 0 {
        let mut row = Vec::new();
        let mut row_meta = RowMeta::default();
        let mut rows: Vec<AlignedRow<'a>> = Vec::new();
        let mut metas: Vec<RowMeta<'a>> = Vec::new();

        for item in preprocessed
            .items
            .into_iter()
            .chain(iter::once(RawMathItem::Linebreak))
        {
            match item {
                RawMathItem::Linebreak => {
                    rows.push(split_at_align(row.drain(..), styles));
                    metas.push(row_meta);
                    row_meta = RowMeta::default();
                }
                RawMathItem::LineMarker(marker) => {
                    // Update row metadata from marker
                    let numbered = marker.numbered.get(styles);
                    row_meta.numbered = match numbered {
                        Smart::Auto => None, // Will be determined later based on global setting
                        Smart::Custom(b) => Some(b), // Explicitly set
                    };
                    row_meta.line_ref = marker.line_ref.get_ref(styles).clone();
                    row_meta.span = Span::detached(); // Markers don't have a meaningful span here
                }
                other => {
                    row.push(other);
                }
            }
        }

        if pad {
            let ncols = rows.iter().map(AlignedRow::len).max().unwrap_or_default();
            for row in &mut rows {
                row.pad_to(ncols, styles);
            }
        }

        GroupResult::Multiline(rows, metas)
    } else {
        GroupResult::Flat(
            preprocessed
                .items
                .into_iter()
                .filter(|item| !matches!(item, RawMathItem::Align | RawMathItem::LineMarker(_)))
                .map(RawMathItem::into_item)
                .collect::<Option<_>>()
                .unwrap(),
        )
    }
}

/// The result of processing items for a table cell.
pub(crate) struct TableCellResult<'a> {
    /// Linebreaks stripped, and items split at alignment points.
    pub sub_columns: AlignedRow<'a>,
    /// Whether the original input contained any linebreaks.
    pub had_linebreaks: bool,
}

/// Processes raw items for a table cell.
pub(crate) fn process_table_cell<'a, I>(
    items: I,
    styles: StyleChain<'a>,
) -> TableCellResult<'a>
where
    I: IntoIterator<Item = RawMathItem<'a>>,
    I::IntoIter: ExactSizeIterator,
{
    let preprocessed = preprocess(items, false, true);
    let sub_columns = if preprocessed.has_align {
        split_at_align(
            preprocessed
                .items
                .into_iter()
                .filter(|item| !matches!(item, RawMathItem::LineMarker(_))),
            styles,
        )
    } else {
        AlignedRow::new(vec![MathItem::wrap(
            preprocessed
                .items
                .into_iter()
                .filter(|item| !matches!(item, RawMathItem::LineMarker(_)))
                .map(RawMathItem::into_item)
                .collect::<Option<_>>()
                .unwrap(),
            styles,
        )])
    };
    TableCellResult {
        sub_columns,
        had_linebreaks: preprocessed.had_linebreaks,
    }
}

/// Internal result of the preprocessing logic.
struct Preprocessed<'a> {
    items: SmallVec<[RawMathItem<'a>; 8]>,
    had_linebreaks: bool,
    has_align: bool,
    linebreaks: u32,
}

/// Takes the given [`RawMathItem`]s and processes the spacing between them.
///
/// The `closing` parameter indicates whether a closing delimiter follows the
/// items. The `strip_linebreaks` parameter indicates whether linebreaks should
/// be discarded.
///
/// The behavior of spacing around alignment points is subtle and differs from
/// the `align` environment in amsmath. The current policy is:
/// > always put the correct spacing between items separated by an alignment
/// > point, and move the spacing between items in different columns of a
/// > (right-aligned, left-aligned) pair to the right-aligned column
///
/// This is handled in the [`split_at_align`] function.
fn preprocess<'a, I>(items: I, closing: bool, strip_linebreaks: bool) -> Preprocessed<'a>
where
    I: IntoIterator<Item = RawMathItem<'a>>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = items.into_iter();
    let mut resolved = MathBuffer::with_capacity(iter.len());

    let mut last: Option<usize> = None;
    let mut space: Option<MathItem> = None;
    let mut had_linebreaks = false;
    let mut has_align = false;
    let mut linebreaks: u32 = 0;

    for item in iter {
        match item {
            // Tags don't affect layout.
            RawMathItem::Item(MathItem::Tag(_)) => {
                resolved.push(item);
                continue;
            }
            // Keep space only if supported by spaced items.
            RawMathItem::Item(MathItem::Space) => {
                if last.is_some() {
                    space = item.into_item();
                }
                continue;
            }

            // Explicit spacing disables automatic spacing.
            RawMathItem::Item(MathItem::Spacing(width, weak)) => {
                last = None;
                space = None;

                if weak {
                    let Some(resolved_last) = resolved.last_mut() else {
                        continue;
                    };
                    if let RawMathItem::Item(MathItem::Spacing(prev, true)) =
                        resolved_last
                    {
                        *prev = (*prev).max(width);
                        continue;
                    }
                }

                resolved.push(item);
                continue;
            }

            // Alignment points are resolved later.
            RawMathItem::Align => {
                has_align = true;
                resolved.push(item);
                continue;
            }

            // Line markers are passed through for later processing.
            RawMathItem::LineMarker(_) => {
                resolved.push(item);
                continue;
            }

            // New line, new things.
            RawMathItem::Linebreak => {
                had_linebreaks = true;
                if strip_linebreaks {
                    continue;
                }
                linebreaks += 1;
                resolved.push(item);
                space = None;
                last = None;
                continue;
            }

            _ => {}
        }

        let mut item = item.into_item().unwrap();

        // Convert variable operators into binary operators if something
        // precedes them and they are not preceded by a operator or comparator.
        if item.class() == MathClass::Vary
            && let Some(RawMathItem::Item(prev)) = last.map(|i| &resolved[i])
            && matches!(
                prev.class(),
                MathClass::Normal
                    | MathClass::Alphabetic
                    | MathClass::Closing
                    | MathClass::Fence
            )
        {
            item.set_class(MathClass::Binary);
        }

        // Insert spacing between the last and this non-ignorant item.
        if !item.is_ignorant() {
            if let Some(i) = last
                && let RawMathItem::Item(ref mut prev) = resolved[i]
                && let Some(s) = spacing(prev, space.take(), &mut item)
            {
                resolved.insert(i + 1, RawMathItem::Item(s));
            }

            last = Some(resolved.len());
        }

        resolved.push(RawMathItem::Item(item));
    }

    // Apply closing punctuation spacing if applicable.
    if closing
        && let Some(RawMathItem::Item(item)) = resolved.last_mut()
        && item.rclass() == MathClass::Punctuation
        && item.size().is_none_or(|s| s > MathSize::Script)
    {
        item.set_rspace(Some(THIN))
    } else if let Some(idx) = resolved.last_index()
        && let RawMathItem::Item(MathItem::Spacing(_, true)) = resolved.0[idx]
    {
        resolved.0.remove(idx);
    }

    // Strip final trailing linebreak.
    if !closing
        && let Some(idx) = resolved.last_index()
        && matches!(resolved.0[idx], RawMathItem::Linebreak)
    {
        resolved.0.remove(idx);
        linebreaks -= 1;
    }

    Preprocessed {
        items: resolved.0,
        had_linebreaks,
        has_align,
        linebreaks,
    }
}

/// Computes the spacing between two adjacent math items.
fn spacing<'a>(
    l: &mut MathItem,
    space: Option<MathItem<'a>>,
    r: &mut MathItem,
) -> Option<MathItem<'a>> {
    use MathClass::*;

    let script = |f: &MathItem| f.size().is_some_and(|s| s <= MathSize::Script);

    match (l.rclass(), r.lclass()) {
        // No spacing before punctuation; thin spacing after punctuation, unless
        // in script size.
        (_, Punctuation) => {}
        (Punctuation, _) if !script(l) => l.set_rspace(Some(THIN)),

        // No spacing after opening delimiters and before closing delimiters.
        (Opening, _) | (_, Closing) => {}

        // Thick spacing around relations, unless followed by a another relation
        // or in script size.
        (Relation, Relation) => {}
        (Relation, _) if !script(l) => l.set_rspace(Some(THICK)),
        (_, Relation) if !script(r) => r.set_lspace(Some(THICK)),

        // Medium spacing around binary operators, unless in script size.
        (Binary, _) if !script(l) => l.set_rspace(Some(MEDIUM)),
        (_, Binary) if !script(r) => r.set_lspace(Some(MEDIUM)),

        // Thin spacing around large operators, unless to the left of
        // an opening delimiter. TeXBook, p170
        (Large, Opening | Fence) => {}
        (Large, _) => l.set_rspace(Some(THIN)),

        (_, Large) => r.set_lspace(Some(THIN)),

        // Spacing around spaced frames.
        _ if (l.is_spaced() || r.is_spaced()) => return space,

        _ => {}
    };

    None
}

/// A wrapper around `SmallVec<[RawMathItem; 8]>` that ignores ignorant items in
/// some access methods.
struct MathBuffer<'a>(SmallVec<[RawMathItem<'a>; 8]>);

impl<'a> MathBuffer<'a> {
    /// Creates a new buffer with the given capacity.
    fn with_capacity(size: usize) -> Self {
        Self(SmallVec::with_capacity(size))
    }

    /// Returns a mutable reference to the last non-ignorant item.
    fn last_mut(&mut self) -> Option<&mut RawMathItem<'a>> {
        self.0.iter_mut().rev().find(|i| !i.is_ignorant())
    }

    /// Returns the physical index of the last non-ignorant item.
    fn last_index(&self) -> Option<usize> {
        self.0.iter().rposition(|i| !i.is_ignorant())
    }
}

impl<'a> Deref for MathBuffer<'a> {
    type Target = SmallVec<[RawMathItem<'a>; 8]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for MathBuffer<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

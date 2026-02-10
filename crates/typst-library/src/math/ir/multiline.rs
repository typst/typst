use bumpalo::{Bump, collections::Vec as BumpVec};

use super::item::*;
use crate::foundations::{Resolve, StyleChain};

/// Builds a multiline item from preprocessed items that contain linebreaks.
pub(super) fn build_multiline<'a>(
    items: BumpVec<'a, MathItem<'a>>,
    styles: StyleChain<'a>,
    bump: &'a Bump,
) -> MathItem<'a> {
    let nrows = items
        .iter()
        .filter(|item| matches!(item, MathItem::Linebreak))
        .count()
        + 1;

    let mut rows: BumpVec<'a, BumpVec<'a, BumpVec<'a, MathItem<'a>>>> =
        BumpVec::with_capacity_in(nrows, bump);

    let mut row = BumpVec::new_in(bump);
    for item in items {
        if matches!(item, MathItem::Linebreak) {
            rows.push(split_at_align(row.drain(..), bump));
        } else {
            row.push(item);
        }
    }
    rows.push(split_at_align(row, bump));

    let row_lengths = bump.alloc_slice_fill_iter(rows.iter().map(|row| row.len()));

    let ncols = row_lengths.iter().copied().max().unwrap_or_default();
    let rows = BumpVec::from_iter_in(
        rows.into_iter().map(|mut row| {
            // Pad rows to have the same number of columns.
            while row.len() < ncols {
                row.push(BumpVec::new_in(bump));
            }

            // Wrap each column's items into a single MathItem.
            BumpVec::from_iter_in(
                row.into_iter().map(|cell| MathItem::wrap(cell, styles)),
                bump,
            )
        }),
        bump,
    );

    MultilineItem::create(rows, row_lengths, styles)
}

/// Splits preprocessed items at alignment point markers into columns, moving
/// spacing between items in different columns of a (right-aligned,
/// left-aligned) pair to the right-aligned column.
pub(crate) fn split_at_align<'a, I>(
    items: I,
    bump: &'a Bump,
) -> BumpVec<'a, BumpVec<'a, MathItem<'a>>>
where
    I: IntoIterator<Item = MathItem<'a>>,
{
    let mut cols = BumpVec::from_iter_in([BumpVec::new_in(bump)], bump);

    let mut at_boundary = false;
    for mut item in items {
        if matches!(item, MathItem::Align) {
            cols.push(BumpVec::new_in(bump));
            at_boundary = true;
            continue;
        }

        // If we just passed an alignment point, check if this item has lspace
        // that should be moved to the previous column.
        if at_boundary && !item.is_ignorant() {
            if cols.len().is_multiple_of(2)
                && let MathItem::Component(ref mut comp) = item
                && let Some(lspace) = comp.props.lspace.take()
            {
                let resolved = lspace.resolve(comp.styles);
                let idx = cols.len() - 2;
                cols[idx].push(MathItem::Spacing(resolved, false));
            }

            at_boundary = false;
        }

        cols.last_mut().unwrap().push(item);
    }

    cols
}

use super::item::*;
use crate::foundations::{Resolve, StyleChain};

/// Builds a multiline item from preprocessed items that contain linebreaks.
pub(super) fn build_multiline<'a>(
    items: Vec<MathItem<'a>>,
    styles: StyleChain<'a>,
) -> MathItem<'a> {
    let nrows = items
        .iter()
        .filter(|item| matches!(item, MathItem::Linebreak))
        .count()
        + 1;

    let mut rows = Vec::with_capacity(nrows);

    let mut row = Vec::new();
    for item in items {
        if matches!(item, MathItem::Linebreak) {
            rows.push(split_at_align(row.drain(..)));
        } else {
            row.push(item);
        }
    }
    rows.push(split_at_align(row));

    let ncols = rows.iter().map(|row| row.len()).max().unwrap_or_default();
    let rows = rows
        .into_iter()
        .map(|mut row| {
            // Pad rows to have the same number of columns.
            while row.len() < ncols {
                row.push(Vec::new());
            }

            // Wrap each column's items into a single MathItem.
            row.into_iter().map(|cell| MathItem::wrap(cell, styles)).collect()
        })
        .collect();

    MultilineItem::create(rows, styles)
}

/// Splits preprocessed items at alignment point markers into columns, moving
/// spacing between items in different columns of a (right-aligned,
/// left-aligned) pair to the right-aligned column.
pub(crate) fn split_at_align<'a, I>(items: I) -> Vec<Vec<MathItem<'a>>>
where
    I: IntoIterator<Item = MathItem<'a>>,
{
    let mut cols = vec![vec![]];

    let mut at_boundary = false;
    for mut item in items {
        if matches!(item, MathItem::Align) {
            cols.push(Vec::new());
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

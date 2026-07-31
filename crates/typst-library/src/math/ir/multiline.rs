use std::vec;

use typst_syntax::Span;
use unicode_math_class::MathClass;

use super::item::{FencedBody, FencedItem, MathItem, RawMathItem, SharedFenceSizing};
use crate::foundations::StyleChain;

/// A row split at alignment points into grouped (single-item) columns.
#[derive(Debug)]
pub struct AlignedRow<'a>(Vec<MathItem<'a>>);

impl<'a> AlignedRow<'a> {
    /// Create a row from aligned columns.
    pub(crate) fn new(columns: Vec<MathItem<'a>>) -> Self {
        Self(columns)
    }

    /// The number of columns in this row.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this row is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Pad the row with empty group items until its length reaches `length`.
    pub(super) fn pad_to(&mut self, length: usize, styles: StyleChain<'a>) {
        while self.0.len() < length {
            self.0.push(MathItem::wrap(Vec::new(), styles));
        }
    }

    /// Returns an iterator over references to the math items.
    pub fn iter(&self) -> std::slice::Iter<'_, MathItem<'a>> {
        self.0.iter()
    }

    /// Returns a reference to the item at the given index.
    pub fn get(&self, index: usize) -> Option<&MathItem<'a>> {
        self.0.get(index)
    }
}

impl<'a> IntoIterator for AlignedRow<'a> {
    type Item = MathItem<'a>;
    type IntoIter = vec::IntoIter<MathItem<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Splits a fenced item's body by alignment points and linebreaks into
/// segments.
pub(super) fn expand_multiline_fence<'a>(
    rows: Vec<AlignedRow<'a>>,
    mut open: Option<MathItem<'a>>,
    mut close: Option<MathItem<'a>>,
    styles: StyleChain<'a>,
    span: Span,
) -> Vec<RawMathItem<'a>> {
    let nrows = rows.len();
    let mut bodies = Vec::new();
    let mut row_lengths = Vec::with_capacity(nrows);

    for row in rows {
        row_lengths.push(row.len());
        bodies.extend(row);
    }

    let ncells: usize = row_lengths.iter().sum();
    let sizing = SharedFenceSizing::new(bodies, styles);

    let mut result = Vec::with_capacity((2 * ncells).saturating_sub(1));
    let mut body_idx = 0;
    for (row_idx, &ncols) in row_lengths.iter().enumerate() {
        if row_idx > 0 {
            result.push(RawMathItem::Linebreak);
        }

        for col_idx in 0..ncols {
            if col_idx > 0 {
                result.push(RawMathItem::Align);
            }

            let is_first = row_idx == 0 && col_idx == 0;
            let is_last = row_idx + 1 == nrows && col_idx + 1 == ncols;
            result.push(
                FencedItem::create(
                    open.take_if(|_| is_first),
                    close.take_if(|_| is_last),
                    FencedBody::shared(body_idx, sizing.clone()),
                    true,
                    styles,
                    span,
                )
                .into(),
            );

            body_idx += 1;
        }
    }

    result
}

/// Splits preprocessed items at alignment point markers into columns, marking
/// items which should have their spacing moved in different columns of a
/// (right-aligned, left-aligned) pair to the right-aligned column.
pub(crate) fn split_at_align<'a, I>(items: I, styles: StyleChain<'a>) -> AlignedRow<'a>
where
    I: IntoIterator<Item = RawMathItem<'a>>,
{
    let mut cols = vec![vec![]];

    let mut at_boundary = false;
    for raw in items {
        match raw {
            RawMathItem::Align => {
                cols.push(Vec::new());
                at_boundary = true;
            }
            RawMathItem::Linebreak => unreachable!(),
            RawMathItem::Item(mut item) => {
                // If we just passed an alignment point, check if this item is
                // semantically infix.
                if at_boundary && !item.is_ignorant() {
                    if cols.len().is_multiple_of(2)
                        && matches!(item.class(), MathClass::Relation | MathClass::Binary)
                        && let MathItem::Component(ref mut comp) = item
                    {
                        comp.props.align_form_infix = true;
                    }

                    at_boundary = false;
                }

                cols.last_mut().unwrap().push(item);
            }
        }
    }

    AlignedRow::new(cols.into_iter().map(|col| MathItem::wrap(col, styles)).collect())
}

use std::cmp::Ordering;

use super::ctx::MathContext;
use super::fragment::MathFragment;
use super::row::MathRow;
use crate::math::LayoutMath;
use crate::prelude::*;

/// A math alignment point: `&`, `&&`.
///
/// Display: Alignment Point
/// Category: math
#[element(LayoutMath)]
pub struct AlignPointElem {}

impl LayoutMath for AlignPointElem {
    fn layout_math(&self, ctx: &mut MathContext<'_, '_, '_>) -> SourceResult<()> {
        ctx.push(MathFragment::Align);
        Ok(())
    }
}

/// Determine the position of the alignment points.
pub(super) fn alignments(rows: &[MathRow]) -> Vec<Abs> {
    let count = rows
        .iter()
        .map(|row| {
            row.iter()
                .filter(|fragment| matches!(fragment, MathFragment::Align))
                .count()
        })
        .max()
        .unwrap_or(0);

    let mut points = vec![Abs::zero(); count];
    for current in 0..count {
        for row in rows {
            let mut x_cursor = Abs::zero();
            let mut point_idx = 0;
            for fragment in row.iter() {
                if matches!(fragment, MathFragment::Align) {
                    match point_idx.cmp(&current) {
                        Ordering::Less => {
                            x_cursor = points[point_idx];
                        }
                        Ordering::Equal => {
                            points[point_idx].set_max(x_cursor);
                        }
                        Ordering::Greater => {}
                    }
                    point_idx += 1;
                }
                x_cursor += fragment.width();
            }
        }
    }

    points
}

use std::cmp::Ordering;

use super::*;

/// A math alignment point: `&`, `&&`.
///
/// Display: Alignment Point
/// Category: math
#[element(LayoutMath)]
pub struct AlignPointElem {}

impl LayoutMath for AlignPointElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
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
            let mut x = Abs::zero();
            let mut i = 0;
            for fragment in row.iter() {
                if matches!(fragment, MathFragment::Align) {
                    match i.cmp(&current) {
                        Ordering::Less => {
                            x = points[i];
                        }
                        Ordering::Equal => {
                            points[i].set_max(x);
                        }
                        _ => {}
                    }
                    i += 1;
                }
                x += fragment.width();
            }
        }
    }

    points
}

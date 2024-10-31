use crate::diag::SourceResult;
use crate::foundations::{elem, Content, NativeElement, Packed, StyleChain};
use crate::layout::Abs;
use crate::math::{LayoutMath, MathContext, MathFragment, MathRun};
use crate::utils::singleton;

/// A math alignment point: `&`, `&&`.
#[elem(title = "Alignment Point", LayoutMath)]
pub struct AlignPointElem {}

impl AlignPointElem {
    /// Get the globally shared alignment point element.
    pub fn shared() -> &'static Content {
        singleton!(Content, AlignPointElem::new().pack())
    }
}

impl LayoutMath for Packed<AlignPointElem> {
    fn layout_math(&self, ctx: &mut MathContext, _: StyleChain) -> SourceResult<()> {
        ctx.push(MathFragment::Align);
        Ok(())
    }
}

pub(super) struct AlignmentResult {
    pub points: Vec<Abs>,
    pub width: Abs,
}

/// Determine the positions of the alignment points, according to the input rows combined.
pub(super) fn alignments(rows: &[MathRun]) -> AlignmentResult {
    let mut widths = Vec::<Abs>::new();

    let mut pending_width = Abs::zero();
    for row in rows {
        let mut width = Abs::zero();
        let mut alignment_index = 0;

        for fragment in row.iter() {
            if matches!(fragment, MathFragment::Align) {
                if alignment_index < widths.len() {
                    widths[alignment_index].set_max(width);
                } else {
                    widths.push(width.max(pending_width));
                }
                width = Abs::zero();
                alignment_index += 1;
            } else {
                width += fragment.width();
            }
        }
        if widths.is_empty() {
            pending_width.set_max(width);
        } else if alignment_index < widths.len() {
            widths[alignment_index].set_max(width);
        } else {
            widths.push(width.max(pending_width));
        }
    }

    let mut points = widths;
    for i in 1..points.len() {
        let prev = points[i - 1];
        points[i] += prev;
    }
    AlignmentResult {
        width: points.last().copied().unwrap_or(pending_width),
        points,
    }
}

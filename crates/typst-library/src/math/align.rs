use super::*;

/// A math alignment point: `&`, `&&`.
#[elem(title = "Alignment Point", LayoutMath)]
pub struct AlignPointElem {}

impl LayoutMath for AlignPointElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.push(MathFragment::Align);
        Ok(())
    }
}

pub(super) struct AlignmentResult {
    pub points: Vec<Abs>,
    pub width: Abs,
}

/// Determine the position of the alignment points.
pub(super) fn alignments(rows: &[MathRow]) -> AlignmentResult {
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

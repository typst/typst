use typst_library::foundations::StyleChain;
use typst_library::layout::{Abs, Em};
use typst_library::text::{FontFamily, TextElem};
use typst_utils::singleton;

use super::MathFragment;

/// How much less high scaled delimiters can be than what they wrap.
pub const DELIM_SHORT_FALL: Em = Em::new(0.1);

/// Resolve a prioritized iterator over the font families for math.
pub fn families(styles: StyleChain<'_>) -> impl Iterator<Item = &'_ FontFamily> + Clone {
    let fallbacks = singleton!(Vec<FontFamily>, {
        [
            "new computer modern math",
            "libertinus serif",
            "twitter color emoji",
            "noto color emoji",
            "apple color emoji",
            "segoe ui emoji",
        ]
        .into_iter()
        .map(FontFamily::new)
        .collect()
    });

    let tail = if styles.get(TextElem::fallback) { fallbacks.as_slice() } else { &[] };
    styles.get_ref(TextElem::font).into_iter().chain(tail.iter())
}

/// Determine the positions of the alignment points, according to the input rows combined.
pub fn alignments(rows: &[Vec<MathFragment>]) -> AlignmentResult {
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

pub struct AlignmentResult {
    pub points: Vec<Abs>,
    pub width: Abs,
}

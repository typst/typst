use super::*;

pub(super) const THIN: Em = Em::new(1.0 / 6.0);
pub(super) const MEDIUM: Em = Em::new(2.0 / 9.0);
pub(super) const THICK: Em = Em::new(5.0 / 18.0);
pub(super) const QUAD: Em = Em::new(1.0);

/// Hook up all spacings.
pub(super) fn define(math: &mut Scope) {
    math.define("thin", HElem::new(THIN.into()).pack());
    math.define("med", HElem::new(MEDIUM.into()).pack());
    math.define("thick", HElem::new(THICK.into()).pack());
    math.define("quad", HElem::new(QUAD.into()).pack());
}

/// Create the spacing between two fragments in a given style.
pub(super) fn spacing(
    l: &MathFragment,
    space: Option<MathFragment>,
    r: &MathFragment,
) -> Option<MathFragment> {
    use MathClass::*;

    let class = |f: &MathFragment| f.class().unwrap_or(Special);
    let resolve = |v: Em, f: &MathFragment| {
        Some(MathFragment::Spacing(f.font_size().map_or(Abs::zero(), |size| v.at(size))))
    };
    let script =
        |f: &MathFragment| f.style().map_or(false, |s| s.size <= MathSize::Script);

    match (class(l), class(r)) {
        // No spacing before punctuation; thin spacing after punctuation, unless
        // in script size.
        (_, Punctuation) => None,
        (Punctuation, _) if !script(l) => resolve(THIN, l),

        // No spacing after opening delimiters and before closing delimiters.
        (Opening, _) | (_, Closing) => None,

        // Thick spacing around relations, unless followed by a another relation
        // or in script size.
        (Relation, Relation) => None,
        (Relation, _) if !script(l) => resolve(THICK, l),
        (_, Relation) if !script(r) => resolve(THICK, r),

        // Medium spacing around binary operators, unless in script size.
        (Binary, _) if !script(l) => resolve(MEDIUM, l),
        (_, Binary) if !script(r) => resolve(MEDIUM, r),

        // Thin spacing around large operators, unless to the left of
        // an opening delimiter. TeXBook, p170
        (Large, Opening | Fence) => None,
        (Large, _) => resolve(THIN, l),
        (_, Large) => resolve(THIN, r),

        // Spacing around spaced frames.
        _ if (l.is_spaced() || r.is_spaced()) => space,

        _ => None,
    }
}

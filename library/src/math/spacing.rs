use super::*;

pub(super) const ZERO: Em = Em::zero();
pub(super) const THIN: Em = Em::new(1.0 / 6.0);
pub(super) const MEDIUM: Em = Em::new(2.0 / 9.0);
pub(super) const THICK: Em = Em::new(5.0 / 18.0);
pub(super) const QUAD: Em = Em::new(1.0);

/// Determine the spacing between two fragments in a given style.
pub(super) fn spacing(left: MathClass, right: MathClass, style: MathStyle) -> Em {
    use MathClass::*;
    let script = style.size <= MathSize::Script;
    match (left, right) {
        // No spacing before punctuation; thin spacing after punctuation, unless
        // in script size.
        (_, Punctuation) => ZERO,
        (Punctuation, _) if !script => THIN,

        // No spacing after opening delimiters and before closing delimiters.
        (Opening, _) | (_, Closing) => ZERO,

        // Thick spacing around relations, unless followed by a another relation
        // or in script size.
        (Relation, Relation) => ZERO,
        (Relation, _) | (_, Relation) if !script => THICK,

        // Medium spacing around binary operators, unless in script size.
        (Vary | Binary, _) | (_, Vary | Binary) if !script => MEDIUM,

        // Thin spacing around large operators, unless next to a delimiter.
        (Large, Opening | Fence) | (Closing | Fence, Large) => ZERO,
        (Large, _) | (_, Large) => THIN,

        _ => ZERO,
    }
}

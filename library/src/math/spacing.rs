use super::*;

pub(super) const ZERO: Em = Em::zero();
pub(super) const THIN: Em = Em::new(1.0 / 6.0);
pub(super) const MEDIUM: Em = Em::new(2.0 / 9.0);
pub(super) const THICK: Em = Em::new(5.0 / 18.0);
pub(super) const QUAD: Em = Em::new(1.0);

/// Hook up all spacings.
pub(super) fn define(math: &mut Scope) {
    math.define("thin", HNode::strong(THIN).pack());
    math.define("med", HNode::strong(MEDIUM).pack());
    math.define("thick", HNode::strong(THICK).pack());
    math.define("quad", HNode::strong(QUAD).pack());
}

/// Determine the spacing between two fragments in a given style.
pub(super) fn spacing(
    left: &MathFragment,
    right: &MathFragment,
    style: MathStyle,
    space: bool,
    space_width: Em,
) -> Em {
    use MathClass::*;
    let script = style.size <= MathSize::Script;
    let class = |frag: &MathFragment| frag.class().unwrap_or(Special);
    match (class(left), class(right)) {
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
        (Binary, _) | (_, Binary) if !script => MEDIUM,

        // Thin spacing around large operators, unless next to a delimiter.
        (Large, Opening | Fence) | (Closing | Fence, Large) => ZERO,
        (Large, _) | (_, Large) => THIN,

        // Spacing around spaced frames.
        _ if space && (is_spaced(left) || is_spaced(right)) => space_width,

        _ => ZERO,
    }
}

/// Whether this fragment should react to adjacent spaces.
fn is_spaced(fragment: &MathFragment) -> bool {
    match fragment {
        MathFragment::Frame(frame) => frame.spaced,
        _ => fragment.class() == Some(MathClass::Fence),
    }
}

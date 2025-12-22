use crate::foundations::elem;
use crate::visualize::Stroke;

/// A horizontal rule.
///
/// Creates a horizontal line that typically spans the full width of its
/// container. Unlike a [`line`], which is a purely visual element, a
/// horizontal rule carries semantic meaning as a thematic break, similar to
/// HTML's `<hr>` element.
///
/// Horizontal rules automatically span the full available width unless placed
/// in an inline context. They are ideal for visually separating sections of
/// content while conveying a semantic boundary.
///
/// # Example
/// ```example
/// Introduction
/// #horizontal-rule()
/// Body
/// ```
///
/// # Styling
/// The appearance of the horizontal rule can be customized using the `stroke`
/// parameter or through set rules.
///
/// ```example
/// #set horizontal-rule(stroke: 2pt + red)
/// First part
/// #horizontal-rule()
/// Second part
/// ```
#[elem]
pub struct HorizontalRuleElem {
    /// How to [stroke] the horizontal rule.
    ///
    /// ```example
    /// #horizontal-rule(stroke: 2pt + blue)
    /// ```
    #[fold]
    pub stroke: Stroke,
}

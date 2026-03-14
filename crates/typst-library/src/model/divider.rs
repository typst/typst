use crate::foundations::{Packed, ShowSet, Smart, StyleChain, Styles, elem};
use crate::layout::{BlockElem, Em};

/// A divider.
///
/// Creates a thematic break that separates sections of content. By default,
/// it renders as a horizontal line, but it can be customized through show
/// rules.
///
/// # Example
/// ```example
/// Introduction
/// #divider()
/// Body
/// ```
///
/// # Styling
/// The divider can be styled through show rules.
///
/// ```example
/// #show divider: block[
///   #line(length: 100%, stroke: 2pt + red)
/// ]
/// First part
/// #divider()
/// Second part
/// ```
#[elem(ShowSet)]
pub struct DividerElem {}

impl ShowSet for Packed<DividerElem> {
    fn show_set(&self, _styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(BlockElem::above, Smart::Custom(Em::new(2.0).into()));
        out.set(BlockElem::below, Smart::Custom(Em::new(2.0).into()));
        out
    }
}

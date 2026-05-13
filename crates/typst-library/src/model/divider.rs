use crate::foundations::{Packed, ShowSet, Smart, StyleChain, Styles, elem};
use crate::layout::{BlockElem, Em, Ratio};
use crate::visualize::{LineElem, Stroke};

/// A thematic break that separates sections of content.
///
/// By default, it renders as a horizontal line, but it can be customized
/// through show rules.
///
/// = Example <example>
/// ```example
/// She left without a word.
/// #divider()
/// Three days later, she returned.
/// ```
///
/// = Styling <styling>
/// The divider can be styled through show rules.
///
/// Since the divider shows as a line by default, you can use a set rule to
/// adjust the line's stroke:
///
/// ```example
/// #show divider: set line(stroke: 2pt + red)
/// First part
/// #divider()
/// Second part
/// ```
///
/// You can also fully replace the divider with custom content like a floral or
/// asterisks, but then you should wrap it in a block to preserve spacing:
///
/// ```example
/// #show divider: set align(center)
/// #show divider: block[∗ ∗ ∗]
/// Chapter 1
/// #divider()
/// Chapter 2
/// ```
#[elem(ShowSet)]
pub struct DividerElem {}

impl ShowSet for Packed<DividerElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(BlockElem::above, Smart::Custom(Em::new(2.0).into()));
        out.set(BlockElem::below, Smart::Custom(Em::new(2.0).into()));
        out.set(LineElem::length, Ratio::one().into());
        out.set(
            LineElem::stroke,
            Stroke {
                thickness: Smart::Custom(Em::new(0.05).into()),
                ..Default::default()
            },
        );
        out
    }
}

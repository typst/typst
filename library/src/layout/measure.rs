use crate::prelude::*;

/// Measure the size of content.
///
/// Display: Measure
/// Category: layout
/// Returns: array
#[func]
pub fn measure(
    /// The content whose size to measure.
    content: Content,
    /// The styles with which to layout the content.
    styles: StyleMap,
) -> Value {
    let pod = Regions::one(Axes::splat(Abs::inf()), Axes::splat(false));
    let styles = StyleChain::new(&styles);
    let frame = content.measure(&mut vm.vt, styles, pod)?.into_frame();
    let Size { x, y } = frame.size();
    Value::Array(array![x, y])
}

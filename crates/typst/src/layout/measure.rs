use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{dict, func, Content, Dict, StyleChain, Styles};
use crate::layout::{Abs, Axes, LayoutMultiple, Regions, Size};

/// Measures the layouted size of content.
///
/// The `measure` function lets you determine the layouted size of content. Note
/// that an infinite space is assumed, therefore the measured height/width may
/// not necessarily match the final height/width of the measured content. If you
/// want to measure in the current layout dimensions, you can combine `measure`
/// and [`layout`]($layout).
///
/// # Example
/// The same content can have a different size depending on the styles that
/// are active when it is layouted. For example, in the example below
/// `[#content]` is of course bigger when we increase the font size.
///
/// ```example
/// #let content = [Hello!]
/// #content
/// #set text(14pt)
/// #content
/// ```
///
/// To do a meaningful measurement, you therefore first need to retrieve the
/// active styles with the [`style`]($style) function. You can then pass them to
/// the `measure` function.
///
/// ```example
/// #let thing(body) = style(styles => {
///   let size = measure(body, styles)
///   [Width of "#body" is #size.width]
/// })
///
/// #thing[Hey] \
/// #thing[Welcome]
/// ```
///
/// The measure function returns a dictionary with the entries `width` and
/// `height`, both of type [`length`]($length).
#[func]
pub fn measure(
    /// The engine.
    engine: &mut Engine,
    /// The content whose size to measure.
    content: Content,
    /// The styles with which to layout the content.
    styles: Styles,
) -> SourceResult<Dict> {
    let pod = Regions::one(Axes::splat(Abs::inf()), Axes::splat(false));
    let styles = StyleChain::new(&styles);
    let frame = content.measure(engine, styles, pod)?.into_frame();
    let Size { x, y } = frame.size();
    Ok(dict! { "width" => x, "height" => y })
}

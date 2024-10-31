use comemo::Tracked;

use crate::diag::{warning, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    dict, func, Content, Context, Dict, Resolve, Smart, StyleChain, Styles,
};
use crate::introspection::{Locator, LocatorLink};
use crate::layout::{layout_frame, Abs, Axes, Length, Region, Size};
use crate::syntax::Span;

/// Measures the layouted size of content.
///
/// The `measure` function lets you determine the layouted size of content.
/// By default an infinite space is assumed, so the measured dimensions may
/// not necessarily match the final dimensions of the content.
/// If you want to measure in the current layout dimensions, you can combine
/// `measure` and [`layout`].
///
/// # Example
/// The same content can have a different size depending on the [context] that
/// it is placed into. In the example below, the `[#content]` is of course
/// bigger when we increase the font size.
///
/// ```example
/// #let content = [Hello!]
/// #content
/// #set text(14pt)
/// #content
/// ```
///
/// For this reason, you can only measure when context is available.
///
/// ```example
/// #let thing(body) = context {
///   let size = measure(body)
///   [Width of "#body" is #size.width]
/// }
///
/// #thing[Hey] \
/// #thing[Welcome]
/// ```
///
/// The measure function returns a dictionary with the entries `width` and
/// `height`, both of type [`length`].
#[func(contextual)]
pub fn measure(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: Tracked<Context>,
    /// The callsite span.
    span: Span,
    /// The width available to layout the content.
    ///
    /// Setting this to `{auto}` indicates infinite available width.
    ///
    /// Note that using the `width` and `height` parameters of this function is
    /// different from measuring a sized [`block`] containing the content. In
    /// the following example, the former will get the dimensions of the inner
    /// content instead of the dimensions of the block.
    ///
    /// ```example
    /// #context measure(lorem(100), width: 400pt)
    ///
    /// #context measure(block(lorem(100), width: 400pt))
    /// ```
    #[named]
    #[default(Smart::Auto)]
    width: Smart<Length>,
    /// The height available to layout the content.
    ///
    /// Setting this to `{auto}` indicates infinite available height.
    #[named]
    #[default(Smart::Auto)]
    height: Smart<Length>,
    /// The content whose size to measure.
    content: Content,
    /// _Compatibility:_ This argument is deprecated. It only exists for
    /// compatibility with Typst 0.10 and lower and shouldn't be used anymore.
    #[default]
    styles: Option<Styles>,
) -> SourceResult<Dict> {
    let styles = match &styles {
        Some(styles) => {
            engine.sink.warn(warning!(
                span, "calling `measure` with a styles argument is deprecated";
                hint: "try removing the styles argument"
            ));
            StyleChain::new(styles)
        }
        None => context.styles().at(span)?,
    };

    // Create a pod region with the available space.
    let pod = Region::new(
        Axes::new(
            width.resolve(styles).unwrap_or(Abs::inf()),
            height.resolve(styles).unwrap_or(Abs::inf()),
        ),
        Axes::splat(false),
    );

    // We put the locator into a special "measurement mode" to ensure that
    // introspection-driven features within the content continue to work. Read
    // the "Dealing with measurement" section of the [`Locator`] docs for more
    // details.
    let here = context.location().at(span)?;
    let link = LocatorLink::measure(here);
    let locator = Locator::link(&link);

    let frame = layout_frame(engine, &content, locator, styles, pod)?;
    let Size { x, y } = frame.size();
    Ok(dict! { "width" => x, "height" => y })
}

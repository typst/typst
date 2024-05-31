use comemo::Tracked;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    dict, func, Content, Context, Dict, Resolve, Smart, StyleChain, Styles,
};
use crate::layout::{Abs, Axes, Length, Regions, Size};
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
/// it is placed into. For example, in the example below `[#content]` is of
/// course bigger when we increase the font size.
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
    /// Defaults to `{auto}`, which denotes an infinite width.
    ///
    /// Using the `width` and `height` parameters of this function is
    /// different from measuring a [`box`] containing the content;
    /// the former will get the dimensions of the inner content
    /// instead of the dimensions of the box:
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
    /// Defaults to `{auto}`, which denotes an infinite height.
    #[named]
    #[default(Smart::Auto)]
    height: Smart<Length>,
    /// The content whose size to measure.
    content: Content,
    /// _Compatibility:_ This argument only exists for compatibility with
    /// Typst 0.10 and lower and shouldn't be used anymore.
    #[default]
    styles: Option<Styles>,
) -> SourceResult<Dict> {
    let styles = match &styles {
        Some(styles) => StyleChain::new(styles),
        None => context.styles().at(span)?,
    };

    let available = Axes::new(
        width.resolve(styles).unwrap_or(Abs::inf()),
        height.resolve(styles).unwrap_or(Abs::inf()),
    );

    let pod = Regions::one(available, Axes::splat(false));
    let frame = content.measure(engine, styles, pod)?.into_frame();
    let Size { x, y } = frame.size();
    Ok(dict! { "width" => x, "height" => y })
}

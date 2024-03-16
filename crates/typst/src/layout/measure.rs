use comemo::Tracked;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{dict, func, Content, Context, Dict, StyleChain, Styles};
use crate::layout::{Abs, Axes, LayoutMultiple, Regions, Size};
use crate::syntax::Span;

/// Measures the layouted size of content.
///
/// The `measure` function lets you determine the layouted size of content. Note
/// that an infinite space is assumed, therefore the measured height/width may
/// not necessarily match the final height/width of the measured content. If you
/// want to measure in the current layout dimensions, you can combine `measure`
/// and [`layout`].
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

    let pod = Regions::one(Axes::splat(Abs::inf()), Axes::splat(false));
    let frame = content.measure(engine, styles, pod)?.into_frame();
    let Size { x, y } = frame.size();
    Ok(dict! { "width" => x, "height" => y })
}

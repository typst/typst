use comemo::Track;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{
    dict, elem, func, Content, Context, Func, NativeElement, Packed, StyleChain,
};
use crate::introspection::Locatable;
use crate::layout::{Fragment, LayoutMultiple, Regions, Size};
use crate::syntax::Span;

/// Provides access to the current outer container's (or page's, if none)
/// dimensions (width and height).
///
/// Accepts a function that receives a single parameter, which is a dictionary
/// with keys `width` and `height`, both of type [`length`]. The function is
/// provided [context], meaning you don't need to use it in combination with the
/// `context` keyword. This is why [`measure`] can be called in the example
/// below.
///
/// ```example
/// #let text = lorem(30)
/// #layout(size => [
///   #let (height,) = measure(
///     block(width: size.width, text),
///   )
///   This text is #height high with
///   the current page width: \
///   #text
/// ])
/// ```
///
/// If the `layout` call is placed inside a box with a width of `{800pt}` and a
/// height of `{400pt}`, then the specified function will be given the argument
/// `{(width: 800pt, height: 400pt)}`. If it is placed directly into the page, it
/// receives the page's dimensions minus its margins. This is mostly useful in
/// combination with [measurement]($measure).
///
/// You can also use this function to resolve [`ratio`] to fixed lengths. This
/// might come in handy if you're building your own layout abstractions.
///
/// ```example
/// #layout(size => {
///   let half = 50% * size.width
///   [Half a page is #half wide.]
/// })
/// ```
///
/// Note that the width or height provided by `layout` will be infinite if the
/// corresponding page dimension is set to `{auto}`.
#[func]
pub fn layout(
    /// The call span of this function.
    span: Span,
    /// A function to call with the outer container's size. Its return value is
    /// displayed in the document.
    ///
    /// The container's size is given as a [dictionary] with the keys `width`
    /// and `height`.
    ///
    /// This function is called once for each time the content returned by
    /// `layout` appears in the document. This makes it possible to generate
    /// content that depends on the dimensions of its container.
    func: Func,
) -> Content {
    LayoutElem::new(func).pack().spanned(span)
}

/// Executes a `layout` call.
#[elem(Locatable, LayoutMultiple)]
struct LayoutElem {
    /// The function to call with the outer container's (or page's) size.
    #[required]
    func: Func,
}

impl LayoutMultiple for Packed<LayoutElem> {
    #[typst_macros::time(name = "layout", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Gets the current region's base size, which will be the size of the
        // outer container, or of the page if there is no such container.
        let Size { x, y } = regions.base();
        let loc = self.location().unwrap();
        let context = Context::new(Some(loc), Some(styles));
        let result = self
            .func()
            .call(engine, context.track(), [dict! { "width" => x, "height" => y }])?
            .display();
        result.layout(engine, styles, regions)
    }
}

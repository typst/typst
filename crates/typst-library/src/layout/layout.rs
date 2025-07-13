use typst_syntax::Span;

use crate::foundations::{elem, func, Content, Func, NativeElement};
use crate::introspection::Locatable;

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
///     width: size.width,
///     text,
///   )
///   This text is #height high with
///   the current page width: \
///   #text
/// ])
/// ```
///
/// Note that the `layout` function forces its contents into a [block]-level
/// container, so placement relative to the page or pagebreaks are not possible
/// within it.
///
/// If the `layout` call is placed inside a box with a width of `{800pt}` and a
/// height of `{400pt}`, then the specified function will be given the argument
/// `{(width: 800pt, height: 400pt)}`. If it is placed directly into the page, it
/// receives the page's dimensions minus its margins. This is mostly useful in
/// combination with [measurement]($measure).
///
/// To retrieve the _remaining_ height of the page rather than its full size,
/// you can wrap your `layout` call in a `{block(height: 1fr)}`. This works
/// because the block automatically grows to fill the remaining space (see the
/// [fraction] documentation for more details).
///
/// ```example
/// #set page(height: 150pt)
///
/// #lorem(20)
///
/// #block(height: 1fr, layout(size => [
///   Remaining height: #size.height
/// ]))
/// ```
///
/// You can also use this function to resolve a [`ratio`] to a fixed length.
/// This might come in handy if you're building your own layout abstractions.
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
#[elem(Locatable)]
pub struct LayoutElem {
    /// The function to call with the outer container's (or page's) size.
    #[required]
    pub func: Func,
}

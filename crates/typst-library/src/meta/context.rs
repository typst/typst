use crate::prelude::*;

/// Provides access to the location of content.
///
/// This is useful in combination with [queries]($func/query),
/// [counters]($func/counter), [state]($func/state), and [links]($func/link).
/// See their documentation for more details.
///
/// ```example
/// #locate(loc => [
///   My location: \
///   #loc.position()!
/// ])
/// ```
///
/// ## Methods
/// ### page()
/// Returns the page number for this location.
///
/// Note that this does not return the value of the [page counter]($func/counter)
/// at this location, but the true page number (starting from one).
///
/// If you want to know the value of the page counter, use
/// `{counter(page).at(loc)}` instead.
///
/// - returns: integer
///
/// ### position()
/// Returns a dictionary with the page number and the x, y position for this
/// location. The page number starts at one and the coordinates are measured
/// from the top-left of the page.
///
/// If you only need the page number, use `page()` instead as it allows Typst
/// to skip unnecessary work.
///
/// - returns: dictionary
///
/// ### page-numbering()
/// Returns the page numbering pattern of the page at this location. This can be
/// used when displaying the page counter in order to obtain the local numbering.
/// This is useful if you are building custom indices or outlines.
///
/// If the page numbering is set to `none` at that location, this function returns `none`.
///
/// - returns: string or function or none
///
/// Display: Locate
/// Category: meta
#[func]
pub fn locate(
    /// A function that receives a `location`. Its return value is displayed
    /// in the document.
    ///
    /// This function is called once for each time the content returned by
    /// `locate` appears in the document. That makes it possible to generate
    /// content that depends on its own location in the document.
    func: Func,
) -> Content {
    LocateElem::new(func).pack()
}

/// Executes a `locate` call.
///
/// Display: Locate
/// Category: special
#[element(Locatable, Show)]
struct LocateElem {
    /// The function to call with the location.
    #[required]
    func: Func,
}

impl Show for LocateElem {
    #[tracing::instrument(name = "LocateElem::show", skip(self, vt))]
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(vt.delayed(|vt| {
            let location = self.0.location().unwrap();
            Ok(self.func().call_vt(vt, [location])?.display())
        }))
    }
}

/// Provides access to active styles.
///
/// The styles are currently opaque and only useful in combination with the
/// [`measure`]($func/measure) function. See its documentation for more details.
/// In the future, the provided styles might also be directly accessed to look
/// up styles defined by [set rules]($styling/#set-rules).
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
/// Display: Style
/// Category: meta
#[func]
pub fn style(
    /// A function to call with the styles. Its return value is displayed
    /// in the document.
    ///
    /// This function is called once for each time the content returned by
    /// `style` appears in the document. That makes it possible to generate
    /// content that depends on the style context it appears in.
    func: Func,
) -> Content {
    StyleElem::new(func).pack()
}

/// Executes a style access.
///
/// Display: Style
/// Category: special
#[element(Show)]
struct StyleElem {
    /// The function to call with the styles.
    #[required]
    func: Func,
}

impl Show for StyleElem {
    #[tracing::instrument(name = "StyleElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.func().call_vt(vt, [styles.to_map()])?.display())
    }
}

/// Provides access to the current outer container's (or page's, if none) size
/// (width and height).
///
/// The given function must accept a single parameter, `size`, which is a
/// dictionary with keys `width` and `height`, both of type
/// [`length`]($type/length).
///

/// ```example
/// #let text = lorem(30)
/// #layout(size => style(styles => [
///   #let (height,) = measure(
///     block(width: size.width, text),
///     styles,
///   )
///   This text is #height high with
///   the current page width: \
///   #text
/// ]))
/// ```
///
/// If the `layout` call is placed inside of a box width a width of `{800pt}`
/// and a height of `{400pt}`, then the specified function will be given the
/// parameter `{(width: 800pt, height: 400pt)}`. If it placed directly into the
/// page it receives the page's dimensions minus its margins. This is mostly
/// useful in combination with [measurement]($func/measure).
///
/// You can also use this function to resolve [`ratio`]($type/ratio) to fixed
/// lengths. This might come in handy if you're building your own layout
/// abstractions.
///
/// ```example
/// #layout(size => {
///   let half = 50% * size.width
///   [Half a page is #half wide.]
/// })
/// ```
///
/// Note that this function will provide an infinite width or height if one of
/// the page width or height is `auto`, respectively.
///
/// Display: Layout
/// Category: meta
#[func]
pub fn layout(
    /// A function to call with the outer container's size. Its return value is
    /// displayed in the document.
    ///
    /// The container's size is given as a [dictionary]($type/dictionary) with
    /// the keys `width` and `height`.
    ///
    /// This function is called once for each time the content returned by
    /// `layout` appears in the document. That makes it possible to generate
    /// content that depends on the size of the container it is inside of.
    func: Func,
) -> Content {
    LayoutElem::new(func).pack()
}

/// Executes a `layout` call.
///
/// Display: Layout
/// Category: special
#[element(Layout)]
struct LayoutElem {
    /// The function to call with the outer container's (or page's) size.
    #[required]
    func: Func,
}

impl Layout for LayoutElem {
    #[tracing::instrument(name = "LayoutElem::layout", skip_all)]
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Gets the current region's base size, which will be the size of the
        // outer container, or of the page if there is no such container.
        let Size { x, y } = regions.base();
        let result = self
            .func()
            .call_vt(vt, [dict! { "width" => x, "height" => y }])?
            .display();
        result.layout(vt, styles, regions)
    }
}

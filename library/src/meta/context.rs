use crate::prelude::*;

/// Provides access to the location of content.
///
/// This is useful in combination with [queries]($func/query),
/// [counters]($func/counter), [state]($func/state), and [links]($func/link).
/// See their documentation for more details.
///
/// ```example
/// #locate(loc => [
///   My locatation: \
///   #loc.position()!
/// ])
/// ```
///
/// ## Methods
/// ### page()
/// Return the page number for this location.
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
/// Return a dictionary with the page number and the x, y position for this
/// location. The page number starts at one and the coordinates are measured
/// from the top-left of the page.
///
/// If you only need the page number, use `page()` instead as it allows Typst
/// to skip unnecessary work.
///
/// - returns: dictionary
///
/// Display: Locate
/// Category: meta
/// Returns: content
#[func]
pub fn locate(
    /// A function that receives a `location`. Its return value is displayed
    /// in the document.
    ///
    /// This function is called once for each time the content returned by
    /// `locate` appears in the document. That makes it possible to generate
    /// content that depends on its own location in the document.
    func: Func,
) -> Value {
    LocateElem::new(func).pack().into()
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
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let location = self.0.location().unwrap();
        Ok(self.func().call_vt(vt, [location.into()])?.display())
    }
}

/// Provides access to active styles.
///
/// The styles are currently opaque and only useful in combination with the
/// [`measure`]($func/measure) function. See its documentation for more details.
/// In the future, the provided styles might also be directly accessed to look
/// up styles defined by [set rules]($styling/#set-rules).
///
/// Display: Style
/// Category: meta
/// Returns: content
#[func]
pub fn style(
    /// A function to call with the styles. Its return value is displayed
    /// in the document.
    ///
    /// This function is called once for each time the content returned by
    /// `style` appears in the document. That makes it possible to generate
    /// content that depends on the style context it appears in.
    func: Func,
) -> Value {
    StyleElem::new(func).pack().into()
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
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.func().call_vt(vt, [styles.to_map().into()])?.display())
    }
}

/// Provides access to the current outer container's (or page's, if none) size (width and height).
///
/// The given function must accept a single parameter, `size`, which is a dictionary with keys
/// `width` and `height`, both having the type [`length`]($type/length).
///
/// That is, if this `layout` call is done inside (for example) a box of size 800pt (width)
/// by 400pt (height), then the specified function will be given the parameter
/// `(width: 800pt, height: 400pt)`.
///
/// If, however, this `layout` call is placed directly on the page, not inside any container,
/// then the function will be given `(width: page_width, height: page_height)`, where `page_width`
/// and `page_height` correspond to the current page's respective dimensions.
///
/// This is useful, for example, to convert a [`ratio`]($type/ratio) value (such as `5%`, `100%`
/// etc.), which are usually based upon the outer container's dimensions (precisely what this
/// function gives), to a fixed length (in `pt`).
///
/// This is also useful if you're trying to make content fit a certain box, and doing certain
/// arithmetic using `pt` (for example, comparing different lengths) is required.
///
/// ```example
/// layout(size => {
///     // work with the width and height of the container we're in
///     // using size.width and size.height
/// })
///
/// layout(size => {
///     // convert 49% (width) to 'pt'
///     // note that "ratio" values are always relative to a certain, possibly arbitrary length,
///     // but it's usually the current container's width or height (e.g., for table columns,
///     // 15% would be relative to the width, but, for rows, it would be relative to the height).
///     let percentage_of_width = (49% / 1%) * 0.01 * size.width
///     // ... use the converted value ...
/// })
///
/// // The following two boxes are equivalent, and will have rectangles sized 200pt and 40pt:
///
/// #box(width: 200pt, height: 40pt, {
///     rect(width: 100%, height: 100%)
/// })
///
/// #box(width: 200pt, height: 40pt, layout(size => {
///     rect(width: size.width, height: size.height)
/// }))
/// ```
///
/// Display: Layout
/// Category: meta
/// Returns: content
#[func]
pub fn layout(
    /// A function to call with the outer container's size. Its return value is displayed
    /// in the document.
    ///
    /// This function is called once for each time the content returned by
    /// `layout` appears in the document. That makes it possible to generate
    /// content that depends on the size of the container it is inside.
    func: Func,
) -> Value {
    LayoutElem::new(func).pack().into()
}

/// Executes a `layout` call.
///
/// Display: Layout
/// Category: special
#[element(Layout)]
struct LayoutElem {
    /// The function to call with the outer container's (or page's) size.
    #[required]
    func: Func
}

impl Layout for LayoutElem {
    fn layout(&self, vt: &mut Vt, styles: StyleChain, regions: Regions) -> SourceResult<Fragment> {
        // Gets the current region's base size, which will be the size of the outer container,
        // or of the page if there is no such container.
        let Size { x, y } = regions.base();
        let size_dict = dict! { "width" => x, "height" => y }.into();

        let result = self.func()
            .call_vt(vt, [size_dict])?  // calls func(size)
            .display();

        result.layout(vt, styles, regions)
    }
}

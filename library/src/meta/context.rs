use crate::prelude::*;

/// Provides access to the location of content.
///
/// This is useful in combination with [queries]($func/query),
/// [counters]($func/counter), [state]($func/state), and [links]($func/link).
/// See their documentation for more details.
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

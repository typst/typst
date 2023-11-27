use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, func, Content, Func, NativeElement, Show, StyleChain};
use crate::introspection::Locatable;

/// Provides access to the location of content.
///
/// This is useful in combination with [queries]($query), [counters]($counter),
/// [state]($state), and [links]($link). See their documentation for more
/// details.
///
/// ```example
/// #locate(loc => [
///   My location: \
///   #loc.position()!
/// ])
/// ```
#[func]
pub fn locate(
    /// A function that receives a [`location`]($location). Its return value is
    /// displayed in the document.
    ///
    /// This function is called once for each time the content returned by
    /// `locate` appears in the document. That makes it possible to generate
    /// content that depends on its own location in the document.
    func: Func,
) -> Content {
    LocateElem::new(func).pack()
}

/// Executes a `locate` call.
#[elem(Locatable, Show)]
struct LocateElem {
    /// The function to call with the location.
    #[required]
    func: Func,
}

impl Show for LocateElem {
    #[tracing::instrument(name = "LocateElem::show", skip(self, engine))]
    fn show(&self, engine: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(engine.delayed(|engine| {
            let location = self.location().unwrap();
            Ok(self.func().call(engine, [location])?.display())
        }))
    }
}

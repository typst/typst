use comemo::{Track, Tracked};

use crate::diag::{warning, HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, Content, Context, Func, LocatableSelector, NativeElement, Packed,
    Show, StyleChain, Value,
};
use crate::introspection::{Locatable, Location};
use crate::syntax::Span;

/// Determines the location of an element in the document.
///
/// Takes a selector that must match exactly one element and returns that
/// element's [`location`]. This location can, in particular, be used to
/// retrieve the physical [`page`]($location.page) number and
/// [`position`]($location.position) (page, x, y) for that element.
///
/// # Examples
/// Locating a specific element:
/// ```example
/// #context [
///   Introduction is at: \
///   #locate(<intro>).position()
/// ]
///
/// = Introduction <intro>
/// ```
///
/// # Compatibility
/// In Typst 0.10 and lower, the `locate` function took a closure that made the
/// current location in the document available (like [`here`] does now). This
/// usage pattern is deprecated. Compatibility with the old way will remain for
/// a while to give package authors time to upgrade. To that effect, `locate`
/// detects whether it received a selector or a user-defined function and
/// adjusts its semantics accordingly. This behaviour will be removed in the
/// future.
#[func(contextual)]
pub fn locate(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: Tracked<Context>,
    /// The span of the `locate` call.
    span: Span,
    /// A selector that should match exactly one element. This element will be
    /// located.
    ///
    /// Especially useful in combination with
    /// - [`here`] to locate the current context,
    /// - a [`location`] retrieved from some queried element via the
    ///   [`location()`]($content.location) method on content.
    selector: LocateInput,
) -> HintedStrResult<LocateOutput> {
    Ok(match selector {
        LocateInput::Selector(selector) => {
            LocateOutput::Location(selector.resolve_unique(engine.introspector, context)?)
        }
        LocateInput::Func(func) => {
            engine.sink.warn(warning!(
                span, "`locate` with callback function is deprecated";
                hint: "use a `context` expression instead"
            ));

            LocateOutput::Content(LocateElem::new(func).pack().spanned(span))
        }
    })
}

/// Compatible input type.
pub enum LocateInput {
    Selector(LocatableSelector),
    Func(Func),
}

cast! {
    LocateInput,
    v: Func => {
        if v.element().is_some() {
            Self::Selector(Value::Func(v).cast()?)
        } else {
            Self::Func(v)
        }
    },
    v: LocatableSelector => Self::Selector(v),
}

/// Compatible output type.
pub enum LocateOutput {
    Location(Location),
    Content(Content),
}

cast! {
    LocateOutput,
    self => match self {
        Self::Location(v) => v.into_value(),
        Self::Content(v) => v.into_value(),
    },
    v: Location => Self::Location(v),
    v: Content => Self::Content(v),
}

/// Executes a `locate` call.
#[elem(Locatable, Show)]
struct LocateElem {
    /// The function to call with the location.
    #[required]
    func: Func,
}

impl Show for Packed<LocateElem> {
    #[typst_macros::time(name = "locate", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let location = self.location().unwrap();
        let context = Context::new(Some(location), Some(styles));
        Ok(self.func().call(engine, context.track(), [location])?.display())
    }
}

use std::num::NonZeroUsize;

use crate::diag::{HintedStrResult, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, func, Cast, Content, Context, Func, LocatableSelector, NativeElement,
    Packed, Show, StyleChain, Value,
};
use crate::introspection::Locatable;
use crate::layout::Position;
use crate::syntax::Span;

/// Determines the physical position of an element in the document.
///
/// Takes a selector that must match exactly one element and returns a
/// dictionary with the page number and the x, y position where this element
/// ends up in the layout. The page number starts at one and the coordinates are
/// measured from the top-left of the page.
///
/// If you want to display the current page number, refer to the documentation
/// of the [`counter`]($counter) type. While `here` can be used to determine the
/// physical page number, typically you want the logical page number that may,
/// for instance, have been reset after a preface.
///
/// # Examples
/// Locating a specific element:
/// ```example
/// #context [
///   Introduction is at: \
///   #locate(<intro>)
/// ]
///
/// = Introduction <intro>
/// ```
///
/// Can be used with [`here`]($here) to retrieve the position of the current
/// context:
/// ```example
/// #context [
///   I am located at
///   #locate(here())
/// ]
/// ```
///
/// # Compatibility
/// In Typst 0.10 and lower, the `locate` function took a closure that made the
/// current location in the document available (like [`here`]($here) does now).
/// Compatibility with the old way will remain for a while to give package
/// authors time to upgrade. To that effect, `locate` detects whether it
/// received a selector or a user-defined function and adjusts its semantics
/// accordingly. This behaviour will be removed in the future.
#[func(contextual)]
pub fn locate(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: &Context,
    /// The span of the `locate` call.
    span: Span,
    /// A selector that should match exactly one element. This element will be
    /// located.
    ///
    /// Especially useful in combination with
    /// - [`here`]($here) to locate the current context,
    /// - a [`location`]($location) retrieved from some queried element via the
    ///   [`location()`]($content.location) method on content.
    selector: LocateInput,
    /// The amount of precision with which to locate.
    ///
    /// If you only need the page number, you can allow Typst to skip
    /// unnecessary work.
    ///
    /// ```example
    /// Page: #context locate(
    ///   here(),
    ///   accuracy: "page",
    /// )
    /// ```
    #[named]
    #[default]
    accuracy: LocateAccuracy,
) -> HintedStrResult<LocateOutput> {
    Ok(match selector {
        LocateInput::Selector(selector) => {
            context.introspect()?;
            let loc = selector.resolve_unique(engine.introspector, context)?;
            match accuracy {
                LocateAccuracy::Page => LocateOutput::Page(engine.introspector.page(loc)),
                LocateAccuracy::Position => {
                    LocateOutput::Position(engine.introspector.position(loc))
                }
            }
        }
        LocateInput::Func(func) => {
            LocateOutput::Content(LocateElem::new(func).pack().spanned(span))
        }
    })
}

/// The precision with which to locate.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum LocateAccuracy {
    /// Provides just the page (returns an integer).
    Page,
    /// Provides the page and x, y position (returns a dictionary).
    #[default]
    Position,
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
    Page(NonZeroUsize),
    Position(Position),
    Content(Content),
}

cast! {
    LocateOutput,
    self => match self {
        Self::Page(v) => v.into_value(),
        Self::Position(v) => v.into_value(),
        Self::Content(v) => v.into_value(),
    },
    v: NonZeroUsize => Self::Page(v),
    v: Position => Self::Position(v),
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
        Ok(self.func().call(engine, &context, [location])?.display())
    }
}

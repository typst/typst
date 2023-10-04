//! Helpful imports for creating library functionality.

#[doc(no_inline)]
pub use std::fmt::{self, Debug, Formatter};
#[doc(no_inline)]
pub use std::num::NonZeroUsize;

#[doc(no_inline)]
pub use comemo::{Track, Tracked, TrackedMut};
#[doc(no_inline)]
pub use ecow::{eco_format, EcoString};
#[doc(no_inline)]
pub use typst::diag::{bail, error, At, Hint, SourceResult, StrResult};
#[doc(no_inline)]
pub use typst::doc::*;
#[doc(no_inline)]
pub use typst::eval::{
    array, cast, dict, format_str, func, scope, ty, Args, Array, Bytes, Cast, Dict,
    FromValue, Func, IntoValue, Repr, Scope, Str, Symbol, Type, Value, Vm,
};
#[doc(no_inline)]
pub use typst::geom::*;
#[doc(no_inline)]
pub use typst::model::{
    elem, Behave, Behaviour, Construct, Content, Element, Finalize, Fold, Introspector,
    Label, Locatable, LocatableSelector, Location, Locator, MetaElem, NativeElement,
    PlainText, Resolve, Selector, Set, Show, StyleChain, StyleVec, Styles, Synthesize,
    Unlabellable, Vt,
};
#[doc(no_inline)]
pub use typst::syntax::{FileId, Span, Spanned};
#[doc(no_inline)]
pub use typst::util::NonZeroExt;
#[doc(no_inline)]
pub use typst::World;

#[doc(no_inline)]
pub use crate::layout::{Fragment, Layout, Regions};
#[doc(no_inline)]
pub use crate::shared::{ContentExt, StylesExt};

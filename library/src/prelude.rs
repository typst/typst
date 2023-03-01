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
pub use typst::diag::{bail, error, At, SourceResult, StrResult};
#[doc(no_inline)]
pub use typst::doc::*;
#[doc(no_inline)]
pub use typst::eval::{
    array, castable, dict, format_str, func, Args, Array, AutoValue, Cast, CastInfo,
    Dict, Func, NoneValue, Str, Symbol, Value, Vm,
};
#[doc(no_inline)]
pub use typst::geom::*;
#[doc(no_inline)]
pub use typst::model::{
    capability, capable, node, Content, Finalize, Fold, Introspector, Label, Node,
    NodeId, Prepare, Resolve, Selector, Show, StabilityProvider, StyleChain, StyleMap,
    StyleVec, Unlabellable, Vt,
};
#[doc(no_inline)]
pub use typst::syntax::{Span, Spanned};
#[doc(no_inline)]
pub use typst::World;

#[doc(no_inline)]
pub use crate::layout::{Fragment, Layout, Regions};
#[doc(no_inline)]
pub use crate::shared::{Behave, Behaviour, ContentExt, StyleMapExt};

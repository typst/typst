//! Helpful imports for creating library functionality.

#[doc(no_inline)]
pub use std::fmt::{self, Debug, Formatter};
#[doc(no_inline)]
pub use std::num::NonZeroUsize;

#[doc(no_inline)]
pub use comemo::{Track, Tracked, TrackedMut};
#[doc(no_inline)]
pub use ecow::{format_eco, EcoString};
#[doc(no_inline)]
pub use typst::diag::{bail, error, At, SourceResult, StrResult};
#[doc(no_inline)]
pub use typst::doc::*;
#[doc(no_inline)]
pub use typst::geom::*;
#[doc(no_inline)]
pub use typst::model::{
    array, capability, capable, castable, dict, format_str, func, node, Args, Array,
    AutoValue, Cast, CastInfo, Content, Dict, Finalize, Fold, Func, Introspector, Label,
    Node, NodeId, NoneValue, Prepare, Resolve, Selector, Show, StabilityProvider, Str,
    StyleChain, StyleMap, StyleVec, Symbol, Unlabellable, Value, Vm, Vt,
};
#[doc(no_inline)]
pub use typst::syntax::{Span, Spanned};
#[doc(no_inline)]
pub use typst::World;

#[doc(no_inline)]
pub use crate::layout::{Fragment, Layout, Regions};
#[doc(no_inline)]
pub use crate::shared::{Behave, Behaviour, ContentExt, StyleMapExt};

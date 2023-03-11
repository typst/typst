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
    array, cast_from_value, cast_to_value, dict, format_str, func, Args, Array, Cast,
    CastInfo, Dict, Func, Never, Str, Symbol, Value, Vm,
};
#[doc(no_inline)]
pub use typst::geom::*;
#[doc(no_inline)]
pub use typst::model::{
    node, Construct, Content, Finalize, Fold, Introspector, Label, Node, NodeId, Resolve,
    Selector, Set, Show, StabilityProvider, StyleChain, StyleMap, StyleVec, Synthesize,
    Unlabellable, Vt,
};
#[doc(no_inline)]
pub use typst::syntax::{Span, Spanned};
#[doc(no_inline)]
pub use typst::World;

#[doc(no_inline)]
pub use crate::layout::{Fragment, Layout, Regions};
#[doc(no_inline)]
pub use crate::shared::{Behave, Behaviour, ContentExt, StyleMapExt};

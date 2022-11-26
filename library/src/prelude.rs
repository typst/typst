//! Helpful imports for creating library functionality.

#[doc(no_inline)]
pub use std::fmt::{self, Debug, Formatter};
#[doc(no_inline)]
pub use std::num::NonZeroUsize;

#[doc(no_inline)]
pub use comemo::Tracked;
#[doc(no_inline)]
pub use typst::diag::{bail, error, with_alternative, At, SourceResult, StrResult};
#[doc(no_inline)]
pub use typst::doc::*;
#[doc(no_inline)]
pub use typst::geom::*;
#[doc(no_inline)]
pub use typst::model::{
    array, capability, castable, dict, dynamic, format_str, node, Args, Array, Cast,
    Content, Dict, Finalize, Fold, Func, Label, Node, NodeId, Resolve, Show, Smart, Str,
    StyleChain, StyleMap, StyleVec, Unlabellable, Value, Vm,
};
#[doc(no_inline)]
pub use typst::syntax::{Span, Spanned};
#[doc(no_inline)]
pub use typst::util::{format_eco, EcoString};
#[doc(no_inline)]
pub use typst::World;

#[doc(no_inline)]
pub use crate::layout::{LayoutBlock, LayoutInline, Regions};
#[doc(no_inline)]
pub use crate::shared::{Behave, Behaviour, ContentExt, StyleMapExt};

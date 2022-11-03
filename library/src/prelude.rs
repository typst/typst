//! Helpful imports for creating library functionality.

pub use std::fmt::{self, Debug, Formatter};
pub use std::hash::Hash;
pub use std::io;
pub use std::num::NonZeroUsize;
pub use std::sync::Arc;

pub use comemo::Tracked;
pub use typst::diag::{
    bail, error, with_alternative, At, FileError, FileResult, SourceError, SourceResult,
    StrResult,
};
pub use typst::frame::*;
pub use typst::geom::*;
pub use typst::model::{
    array, capability, castable, dict, dynamic, format_str, node, Args, Array,
    Capability, Cast, Content, Dict, Dynamic, Fold, Func, Key, LangItems, Node, Resolve,
    Scope, Selector, Show, Smart, Str, StyleChain, StyleMap, StyleVec, Value, Vm,
};
pub use typst::syntax::{Span, Spanned};
pub use typst::util::{format_eco, EcoString};
pub use typst::World;

pub use super::ext::{ContentExt, StyleMapExt};
pub use super::layout::{Layout, LayoutBlock, LayoutInline, Regions};
pub use super::text::{FallbackList, TextNode};

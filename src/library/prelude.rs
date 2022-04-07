//! Helpful imports for creating library functionality.

pub use std::fmt::{self, Debug, Formatter};
pub use std::hash::Hash;
pub use std::num::NonZeroUsize;
pub use std::sync::Arc;

pub use typst_macros::node;

pub use crate::diag::{with_alternative, At, Error, StrResult, TypError, TypResult};
pub use crate::eval::{
    Arg, Args, Array, Cast, Content, Dict, Fold, Func, Key, Layout, LayoutNode, Merge,
    Node, Regions, Scope, Show, ShowNode, Smart, StyleChain, StyleMap, StyleVec, Value,
};
pub use crate::frame::*;
pub use crate::geom::*;
pub use crate::syntax::{Span, Spanned};
pub use crate::util::{EcoString, OptionExt};
pub use crate::Context;

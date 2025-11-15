use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use comemo::{Tracked, TrackedMut};
use typst_syntax::{Span, SyntaxMode};
use typst_utils::LazyHash;

use crate::World;
use crate::diag::SourceResult;
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Args, Closure, Content, Context, Func, Module, NativeRuleMap, Scope, StyleChain,
    Styles, Value,
};
use crate::introspection::{Introspector, Locator, SplitLocator};
use crate::layout::{Frame, Region};
use crate::model::DocumentInfo;
use crate::visualize::Color;

/// Defines the `Routines` struct.
macro_rules! routines {
    ($(
        $(#[$attr:meta])*
        fn $name:ident $(<$($time:lifetime),*>)? ($($args:tt)*) -> $ret:ty
    )*) => {
        /// Defines implementation of various Typst compiler routines as a table
        /// of function pointers.
        ///
        /// This is essentially dynamic linking and done to allow for crate
        /// splitting.
        pub struct Routines {
            /// Native show rules.
            pub rules: NativeRuleMap,
            $(
                $(#[$attr])*
                pub $name: $(for<$($time),*>)? fn ($($args)*) -> $ret
            ),*
        }

        impl Hash for Routines {
            fn hash<H: Hasher>(&self, _: &mut H) {}
        }

        impl Debug for Routines {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.pad("Routines(..)")
            }
        }
    };
}

routines! {
    /// Evaluates a string as code and return the resulting value.
    fn eval_string(
        routines: &Routines,
        world: Tracked<dyn World + '_>,
        sink: TrackedMut<Sink>,
        introspector: Tracked<Introspector>,
        context: Tracked<Context>,
        string: &str,
        span: Span,
        mode: SyntaxMode,
        scope: Scope,
    ) -> SourceResult<Value>

    /// Call the closure in the context with the arguments.
    fn eval_closure(
        func: &Func,
        closure: &LazyHash<Closure>,
        routines: &Routines,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
        traced: Tracked<Traced>,
        sink: TrackedMut<Sink>,
        route: Tracked<Route>,
        context: Tracked<Context>,
        args: Args,
    ) -> SourceResult<Value>

    /// Realizes content into a flat list of well-known, styled items.
    fn realize<'a>(
        kind: RealizationKind,
        engine: &mut Engine,
        locator: &mut SplitLocator,
        arenas: &'a Arenas,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<Vec<Pair<'a>>>

    /// Lays out content into a single region, producing a single frame.
    fn layout_frame(
        engine: &mut Engine,
        content: &Content,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Constructs the `html` module.
    fn html_module() -> Module

    /// Wraps content in a span with a color.
    ///
    /// This is a temporary workaround until `TextElem::fill` is supported in
    /// HTML export.
    fn html_span_filled(content: Content, color: Color) -> Content
}

/// Defines what kind of realization we are performing.
pub enum RealizationKind<'a> {
    /// This the root realization for layout. Requires a mutable reference
    /// to document metadata that will be filled from `set document` rules.
    LayoutDocument { info: &'a mut DocumentInfo },
    /// A nested realization in a container (e.g. a `block`). Requires a mutable
    /// reference to an enum that will be set to `FragmentKind::Inline` if the
    /// fragment's content was fully inline.
    LayoutFragment { kind: &'a mut FragmentKind },
    /// A nested realization in a paragraph (i.e. a `par`)
    LayoutPar,
    /// This the root realization for HTML. Requires a mutable reference to
    /// document metadata that will be filled from `set document` rules.
    ///
    /// The `is_inline` function checks whether content consists of an inline
    /// HTML element. It's used by the `PAR` grouping rules. This is slightly
    /// hacky and might be replaced by a mechanism to supply the grouping rules
    /// as a realization user.
    HtmlDocument { info: &'a mut DocumentInfo, is_inline: fn(&Content) -> bool },
    /// A nested realization in a container (e.g. a `block`). Requires a mutable
    /// reference to an enum that will be set to `FragmentKind::Inline` if the
    /// fragment's content was fully inline.
    HtmlFragment { kind: &'a mut FragmentKind, is_inline: fn(&Content) -> bool },
    /// A realization within math.
    Math,
}

impl RealizationKind<'_> {
    /// It this a realization for HTML export?
    pub fn is_html(&self) -> bool {
        matches!(self, Self::HtmlDocument { .. } | Self::HtmlFragment { .. })
    }

    /// It this a realization for a container?
    pub fn is_fragment(&self) -> bool {
        matches!(self, Self::LayoutFragment { .. } | Self::HtmlFragment { .. })
    }

    /// It this a realization for the whole document?
    pub fn is_document(&self) -> bool {
        matches!(self, Self::LayoutDocument { .. } | Self::HtmlDocument { .. })
    }

    /// If this is a document-level realization, accesses the document info.
    pub fn as_document_mut(&mut self) -> Option<&mut DocumentInfo> {
        match self {
            Self::LayoutDocument { info } | Self::HtmlDocument { info, .. } => {
                Some(*info)
            }
            _ => None,
        }
    }

    /// If this is a container-level realization, accesses the fragment kind.
    pub fn as_fragment_mut(&mut self) -> Option<&mut FragmentKind> {
        match self {
            Self::LayoutFragment { kind } | Self::HtmlFragment { kind, .. } => {
                Some(*kind)
            }
            _ => None,
        }
    }
}

/// The kind of fragment output that realization produced.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum FragmentKind {
    /// The fragment's contents were fully inline, and as a result, the output
    /// elements are too.
    Inline,
    /// The fragment contained non-inline content, so inline content was forced
    /// into paragraphs, and as a result, the output elements are not inline.
    Block,
}

/// Temporary storage arenas for lifetime extension during realization.
///
/// Must be kept live while the content returned from realization is processed.
#[derive(Default)]
pub struct Arenas {
    /// A typed arena for owned content.
    pub content: typed_arena::Arena<Content>,
    /// A typed arena for owned styles.
    pub styles: typed_arena::Arena<Styles>,
    /// An untyped arena for everything that is `Copy`.
    pub bump: bumpalo::Bump,
}

/// A pair of content and a style chain that applies to it.
pub type Pair<'a> = (&'a Content, StyleChain<'a>);

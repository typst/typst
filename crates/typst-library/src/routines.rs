use std::hash::{Hash, Hasher};

use comemo::{Tracked, TrackedMut};
use typst_syntax::{Span, SyntaxMode};
use typst_utils::LazyHash;

use crate::diag::SourceResult;
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Args, Cast, Closure, Content, Context, Func, Packed, Scope, StyleChain, Styles, Value,
};
use crate::introspection::{Introspector, Locator, SplitLocator};
use crate::layout::{Frame, Region};
use crate::model::DocumentInfo;
use crate::World;

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
            $(
                $(#[$attr])*
                pub $name: $(for<$($time),*>)? fn ($($args)*) -> $ret
            ),*
        }

        impl Hash for Routines {
            fn hash<H: Hasher>(&self, _: &mut H) {}
        }
    };
}

routines! {
    /// Evaluates a string as code and return the resulting value.
    fn eval_string(
        routines: &Routines,
        world: Tracked<dyn World + '_>,
        sink: TrackedMut<Sink>,
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
}

/// Defines what kind of realization we are performing.
pub enum RealizationKind<'a> {
    /// This the root realization for layout. Requires a mutable reference
    /// to document metadata that will be filled from `set document` rules.
    LayoutDocument(&'a mut DocumentInfo),
    /// A nested realization in a container (e.g. a `block`). Requires a mutable
    /// reference to an enum that will be set to `FragmentKind::Inline` if the
    /// fragment's content was fully inline.
    LayoutFragment(&'a mut FragmentKind),
    /// A nested realization in a paragraph (i.e. a `par`)
    LayoutPar,
    /// This the root realization for HTML. Requires a mutable reference
    /// to document metadata that will be filled from `set document` rules.
    HtmlDocument(&'a mut DocumentInfo),
    /// A nested realization in a container (e.g. a `block`). Requires a mutable
    /// reference to an enum that will be set to `FragmentKind::Inline` if the
    /// fragment's content was fully inline.
    HtmlFragment(&'a mut FragmentKind),
    /// A realization within math.
    Math,
}

impl RealizationKind<'_> {
    /// It this a realization for HTML export?
    pub fn is_html(&self) -> bool {
        matches!(self, Self::HtmlDocument(_) | Self::HtmlFragment(_))
    }

    /// It this a realization for a container?
    pub fn is_fragment(&self) -> bool {
        matches!(self, Self::LayoutFragment(_) | Self::HtmlFragment(_))
    }

    /// If this is a document-level realization, accesses the document info.
    pub fn as_document_mut(&mut self) -> Option<&mut DocumentInfo> {
        match self {
            Self::LayoutDocument(info) | Self::HtmlDocument(info) => Some(*info),
            _ => None,
        }
    }

    /// If this is a container-level realization, accesses the fragment kind.
    pub fn as_fragment_mut(&mut self) -> Option<&mut FragmentKind> {
        match self {
            Self::LayoutFragment(kind) | Self::HtmlFragment(kind) => Some(*kind),
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

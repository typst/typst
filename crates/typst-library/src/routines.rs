#![allow(unused)]

use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

use comemo::{Tracked, TrackedMut};
use typst_syntax::Span;
use typst_utils::LazyHash;

use crate::diag::SourceResult;
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Args, Cast, Closure, Content, Context, Func, Packed, Scope, StyleChain, StyleVec,
    Styles, Value,
};
use crate::introspection::{Introspector, Locator, SplitLocator};
use crate::layout::{
    Abs, BoxElem, ColumnsElem, Fragment, Frame, GridElem, InlineItem, MoveElem, PadElem,
    Region, Regions, Rel, RepeatElem, RotateElem, ScaleElem, Size, SkewElem, StackElem,
};
use crate::math::EquationElem;
use crate::model::{Document, DocumentInfo, EnumElem, ListElem, TableElem};
use crate::visualize::{
    CircleElem, EllipseElem, ImageElem, LineElem, PathElem, PolygonElem, RectElem,
    SquareElem,
};
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
        string: &str,
        span: Span,
        mode: EvalMode,
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

    /// Layout content into a document.
    fn layout_document(
        engine: &mut Engine,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<Document>

    /// Lays out content into multiple regions.
    fn layout_fragment(
        engine: &mut Engine,
        content: &Content,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out content into a single region, producing a single frame.
    fn layout_frame(
        engine: &mut Engine,
        content: &Content,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out inline content.
    fn layout_inline(
        engine: &mut Engine,
        children: &StyleVec,
        locator: Locator,
        styles: StyleChain,
        consecutive: bool,
        region: Size,
        expand: bool,
    ) -> SourceResult<Fragment>

    /// Lays out a [`BoxElem`].
    fn layout_box(
        elem: &Packed<BoxElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Size,
    ) -> SourceResult<Frame>

    /// Lays out a [`ListElem`].
    fn layout_list(
        elem: &Packed<ListElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out an [`EnumElem`].
    fn layout_enum(
        elem: &Packed<EnumElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out a [`GridElem`].
    fn layout_grid(
        elem: &Packed<GridElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out a [`TableElem`].
    fn layout_table(
        elem: &Packed<TableElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out a [`StackElem`].
    fn layout_stack(
        elem: &Packed<StackElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out a [`ColumnsElem`].
    fn layout_columns(
        elem: &Packed<ColumnsElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out a [`MoveElem`].
    fn layout_move(
        elem: &Packed<MoveElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`RotateElem`].
    fn layout_rotate(
        elem: &Packed<RotateElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`ScaleElem`].
    fn layout_scale(
        elem: &Packed<ScaleElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`SkewElem`].
    fn layout_skew(
        elem: &Packed<SkewElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`RepeatElem`].
    fn layout_repeat(
        elem: &Packed<RepeatElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`PadElem`].
    fn layout_pad(
        elem: &Packed<PadElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>

    /// Lays out a [`LineElem`].
    fn layout_line(
        elem: &Packed<LineElem>,
        _: &mut Engine,
        _: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`PathElem`].
    fn layout_path(
        elem: &Packed<PathElem>,
        _: &mut Engine,
        _: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`PolygonElem`].
    fn layout_polygon(
        elem: &Packed<PolygonElem>,
        _: &mut Engine,
        _: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`RectElem`].
    fn layout_rect(
        elem: &Packed<RectElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`SquareElem`].
    fn layout_square(
        elem: &Packed<SquareElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`EllipseElem`].
    fn layout_ellipse(
        elem: &Packed<EllipseElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out a [`CircleElem`].
    fn layout_circle(
        elem: &Packed<CircleElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out an [`ImageElem`].
    fn layout_image(
        elem: &Packed<ImageElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Region,
    ) -> SourceResult<Frame>

    /// Lays out an [`EquationElem`] in a paragraph.
    fn layout_equation_inline(
        elem: &Packed<EquationElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        region: Size,
    ) -> SourceResult<Vec<InlineItem>>

    /// Lays out an [`EquationElem`] in a flow.
    fn layout_equation_block(
        elem: &Packed<EquationElem>,
        engine: &mut Engine,
        locator: Locator,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment>
}

/// In which mode to evaluate a string.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum EvalMode {
    /// Evaluate as code, as after a hash.
    Code,
    /// Evaluate as markup, like in a Typst file.
    Markup,
    /// Evaluate as math, as in an equation.
    Math,
}

/// Defines what kind of realization we are performing.
pub enum RealizationKind<'a> {
    /// This the root realization for the document. Requires a mutable reference
    /// to document metadata that will be filled from `set document` rules.
    Root(&'a mut DocumentInfo),
    /// A nested realization in a container (e.g. a `block`).
    Container,
    /// A realization within math.
    Math,
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

//! Layout of content into a [`Frame`] or [`Fragment`].

mod balance;
mod collect;
mod compose;
mod distribute;

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::rc::Rc;

use bumpalo::Bump;
use comemo::{Track, Tracked, TrackedMut};
use ecow::EcoVec;

use self::balance::Balancer;
pub use self::balance::Mode as BalanceMode;
use self::collect::{
    collect, Child, LineChild, MultiChild, MultiSpill, PlacedChild, SingleChild,
};
use self::compose::{compose, Composer};
use self::distribute::distribute;
use crate::diag::{bail, At, SourceDiagnostic, SourceResult};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{Content, Packed, StyleChain};
use crate::introspection::{
    Introspector, Location, Locator, LocatorLink, SplitLocator, Tag,
};
use crate::layout::{
    Abs, Dir, Fragment, Frame, PlacementScope, Region, Regions, Rel, Size,
};
use crate::model::{FootnoteElem, FootnoteEntry};
use crate::realize::{realize, Arenas, Pair, RealizationKind};
use crate::text::TextElem;
use crate::utils::{NonZeroExt, Numeric};
use crate::World;

use super::Axes;

/// Lays out content into multiple regions.
///
/// When laying out into just one region, prefer [`layout_frame`].
pub fn layout_fragment(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    layout_fragment_impl(
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
        regions,
        NonZeroUsize::ONE,
        Rel::zero(),
        BalanceMode::PackStart,
    )
}

/// Lays out content into regions with columns.
///
/// This is different from just laying out into column-sized regions as the
/// columns can interact due to parent-scoped placed elements.
#[allow(clippy::too_many_arguments)]
pub fn layout_fragment_with_columns(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
    count: NonZeroUsize,
    gutter: Rel<Abs>,
    balance: BalanceMode,
) -> SourceResult<Fragment> {
    layout_fragment_impl(
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
        regions,
        count,
        gutter,
        balance,
    )
}

/// Lays out content into a single region, producing a single frame.
pub fn layout_frame(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    region: Region,
) -> SourceResult<Frame> {
    layout_fragment(engine, content, locator, styles, region.into())
        .map(Fragment::into_frame)
}

/// The internal implementation of [`layout_fragment`].
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_fragment_impl(
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Tracked<Locator>,
    styles: StyleChain,
    regions: Regions,
    columns: NonZeroUsize,
    column_gutter: Rel<Abs>,
    balance: BalanceMode,
) -> SourceResult<Fragment> {
    if !regions.size.x.is_finite() && regions.expand.x {
        bail!(content.span(), "cannot expand into infinite width");
    }
    if !regions.size.y.is_finite() && regions.expand.y {
        bail!(content.span(), "cannot expand into infinite height");
    }

    let link = LocatorLink::new(locator);
    let mut locator = Locator::link(&link).split();
    let mut engine = Engine {
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
    };

    engine.route.check_layout_depth().at(content.span())?;

    let arenas = Arenas::default();
    let children = realize(
        RealizationKind::Container,
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    layout_flow(
        &mut engine,
        &children,
        &mut locator,
        styles,
        regions,
        columns,
        column_gutter,
        false,
        balance,
    )
}

/// Lays out realized content into regions, potentially with columns.
#[allow(clippy::too_many_arguments)]
pub(crate) fn layout_flow(
    engine: &mut Engine,
    children: &[Pair],
    locator: &mut SplitLocator,
    shared: StyleChain,
    mut regions: Regions,
    columns: NonZeroUsize,
    column_gutter: Rel<Abs>,
    root: bool,
    balance: BalanceMode,
) -> SourceResult<Fragment> {
    // Prepare configuration that is shared across the whole flow.
    let config = Config {
        root,
        shared,
        columns: {
            let mut count = columns.get();
            if !regions.size.x.is_finite() {
                count = 1;
            }

            let gutter = column_gutter.relative_to(regions.base().x);
            let width = (regions.size.x - gutter * (count - 1) as f64) / count as f64;
            let dir = TextElem::dir_in(shared);
            ColumnConfig { count, width, gutter, dir }
        },
        footnote: FootnoteConfig {
            separator: FootnoteEntry::separator_in(shared),
            clearance: FootnoteEntry::clearance_in(shared),
            gap: FootnoteEntry::gap_in(shared),
            expand: regions.expand.x,
        },
    };

    // Collect the elements into pre-processed children. These are much easier
    // to handle than the raw elements.
    let bump = Bump::new();
    let children = collect(
        engine,
        &bump,
        children,
        locator.next(&()),
        Size::new(config.columns.width, regions.full),
        regions.expand.x,
    )?;

    let balancer = Balancer::new(&children, &config, balance);

    let mut balaced_bounds = regions.base();
    balaced_bounds.y *= Into::<usize>::into(columns) as f64;
    let mut balancer = balancer.measure(
        engine,
        locator.next(&()),
        Region::new(balaced_bounds, regions.expand),
    )?;
    let mut finished = vec![];

    // This loop runs once per region produced by the flow layout.
    loop {
        let mut work = balancer.borrow_work(regions);
        let frame = compose(engine, &mut work, &config, locator.next(&()), regions)?;
        finished.push(frame);

        // Terminate the loop when everything is processed, though draining the
        // backlog if necessary.
        if work.done() && (!regions.expand.y || regions.backlog.is_empty()) {
            break;
        }

        regions.next();
    }

    Ok(Fragment::frames(finished))
}

/// The work that is left to do by flow layout.
///
/// The lifetimes 'a and 'b are used across flow layout:
/// - 'a is that of the content coming out of realization
/// - 'b is that of the collected/prepared children
#[derive(Clone)]
struct Work<'a, 'b> {
    /// Children that we haven't processed yet. This slice shrinks over time.
    children: &'b [Child<'a>],
    /// Leftovers from a breakable block.
    spill: Option<MultiSpill<'a, 'b>>,
    /// Queued floats that didn't fit in previous regions.
    floats: EcoVec<&'b PlacedChild<'a>>,
    /// Queued footnotes that didn't fit in previous regions.
    footnotes: EcoVec<Packed<FootnoteElem>>,
    /// Spilled frames of a footnote that didn't fully fit. Similar to `spill`.
    footnote_spill: Option<std::vec::IntoIter<Frame>>,
    /// Queued tags that will be attached to the next frame.
    tags: EcoVec<&'a Tag>,
    /// Identifies floats and footnotes that can be skipped if visited because
    /// they were already handled and incorporated as column or page level
    /// insertions.
    skips: Rc<HashSet<Skip>>,
}

/// Identifies an element that that can be skipped if visited because it was
/// already processed.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Skip {
    /// Uniquely identifies a placed elements. We can't use a [`Location`]
    /// because `PlaceElem` is not currently locatable.
    Placed(usize),
    /// Uniquely identifies a footnote.
    Footnote(Location),
}

impl<'a, 'b> Work<'a, 'b> {
    /// Create the initial work state from a list of children.
    fn new(children: &'b [Child<'a>]) -> Self {
        Self {
            children,
            spill: None,
            floats: EcoVec::new(),
            footnotes: EcoVec::new(),
            footnote_spill: None,
            tags: EcoVec::new(),
            skips: Rc::new(HashSet::new()),
        }
    }

    /// Get the first unprocessed child, from the start of the slice.
    fn head(&self) -> Option<&'b Child<'a>> {
        self.children.first()
    }

    /// Mark the `head()` child as processed, advancing the slice by one.
    fn advance(&mut self) {
        self.children = &self.children[1..];
    }

    /// Whether all work is done. This means we can terminate flow layout.
    fn done(&self) -> bool {
        self.children.is_empty()
            && self.spill.is_none()
            && self.floats.is_empty()
            && self.footnote_spill.is_none()
            && self.footnotes.is_empty()
    }

    /// Add skipped floats and footnotes from the insertion areas to the skip
    /// set.
    fn extend_skips(&mut self, skips: &[Skip]) {
        if !skips.is_empty() {
            Rc::make_mut(&mut self.skips).extend(skips.iter().copied());
        }
    }
}

/// Shared configuration for the whole flow.
struct Config<'x> {
    /// Whether this is the root flow, which can host footnotes and line
    /// numbers.
    root: bool,
    /// The styles shared by the whole flow. This is used for footnotes and line
    /// numbers.
    shared: StyleChain<'x>,
    /// Settings for columns.
    columns: ColumnConfig,
    /// Settings for footnotes.
    footnote: FootnoteConfig,
}

/// Configuration of footnotes.
struct FootnoteConfig {
    /// The separator between flow content and footnotes. Typically a line.
    separator: Content,
    /// The amount of space left above the separator.
    clearance: Abs,
    /// The gap between footnote entries.
    gap: Abs,
    /// Whether horizontal expansion is enabled for footnotes.
    expand: bool,
}

/// Configuration of columns.
struct ColumnConfig {
    /// The number of columns.
    count: usize,
    /// The width of each column.
    width: Abs,
    /// The amount of space between columns.
    gutter: Abs,
    /// The horizontal direction in which columns progress. Defined by
    /// `text.dir`.
    dir: Dir,
}

/// The result type for flow layout.
///
/// The `Err(_)` variant incorporate control flow events for finishing and
/// relayouting regions.
type FlowResult<T> = Result<T, Stop>;

/// A control flow event during flow layout.
enum Stop {
    /// Indicates that the current subregion should be finished. Can be caused
    /// by a lack of space (`false`) or an explicit column break (`true`).
    Finish(bool),
    /// Indicates that the given scope should be relayouted.
    Relayout(PlacementScope),
    /// A fatal error.
    Error(EcoVec<SourceDiagnostic>),
}

impl From<EcoVec<SourceDiagnostic>> for Stop {
    fn from(error: EcoVec<SourceDiagnostic>) -> Self {
        Stop::Error(error)
    }
}

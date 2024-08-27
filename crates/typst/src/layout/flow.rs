//! Layout of content
//! - at the top-level, into a [`Document`].
//! - inside of a container, into a [`Frame`] or [`Fragment`].

use std::collections::HashSet;
use std::num::NonZeroUsize;

use comemo::{Track, Tracked, TrackedMut};

use crate::diag::{bail, At, SourceResult};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Content, NativeElement, Packed, Resolve, Smart, StyleChain, Styles,
};
use crate::introspection::{
    Counter, CounterDisplayElem, CounterKey, Introspector, Location, Locator,
    LocatorLink, ManualPageCounter, SplitLocator, Tag, TagElem, TagKind,
};
use crate::layout::{
    Abs, AlignElem, Alignment, Axes, Binding, BlockElem, ColbreakElem, ColumnsElem, Dir,
    FixedAlignment, FlushElem, Fr, Fragment, Frame, FrameItem, HAlignment, Length,
    OuterVAlignment, Page, PageElem, PagebreakElem, Paper, Parity, PlaceElem, Point,
    Ratio, Region, Regions, Rel, Sides, Size, Spacing, VAlignment, VElem,
};
use crate::model::{Document, FootnoteElem, FootnoteEntry, Numbering, ParElem};
use crate::realize::{first_span, realize_root, realizer_container, Arenas, Pair};
use crate::syntax::Span;
use crate::text::TextElem;
use crate::utils::{NonZeroExt, Numeric};
use crate::visualize::Paint;
use crate::World;

/// An item in page layout.
enum PageItem<'a> {
    /// A page run containing content. All runs will be layouted in parallel.
    Run(&'a [Pair<'a>], StyleChain<'a>, Locator<'a>),
    /// Tags in between pages. These will be preprended to the first start of
    /// the next page, or appended at the very end of the final page if there is
    /// no next page.
    Tags(&'a [Pair<'a>]),
    /// An instruction to possibly add a page to bring the page number parity to
    /// the desired state. Can only be done at the end, sequentially, because it
    /// requires knowledge of the concrete page number.
    Parity(Parity, StyleChain<'a>, Locator<'a>),
}

/// A mostly finished layout for one page. Needs only knowledge of its exact
/// page number to be finalized into a `Page`. (Because the margins can depend
/// on the page number.)
#[derive(Clone)]
struct LayoutedPage {
    inner: Frame,
    margin: Sides<Abs>,
    binding: Binding,
    two_sided: bool,
    header: Option<Frame>,
    footer: Option<Frame>,
    background: Option<Frame>,
    foreground: Option<Frame>,
    fill: Smart<Option<Paint>>,
    numbering: Option<Numbering>,
}

/// Layout content into a document.
///
/// This first performs root-level realization and then lays out the resulting
/// elements. In contrast to [`layout_fragment`], this does not take regions
/// since the regions are defined by the page configuration in the content and
/// style chain.
#[typst_macros::time(name = "document")]
pub fn layout_document(
    engine: &mut Engine,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<Document> {
    layout_document_impl(
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        styles,
    )
}

/// The internal implementation of `layout_document`.
#[comemo::memoize]
fn layout_document_impl(
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<Document> {
    let mut locator = Locator::root().split();
    let mut engine = Engine {
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route).unnested(),
    };

    // Mark the external styles as "outside" so that they are valid at the page
    // level.
    let styles = styles.to_map().outside();
    let styles = StyleChain::new(&styles);

    let arenas = Arenas::default();
    let (mut children, info) =
        realize_root(&mut engine, &mut locator, &arenas, content, styles)?;

    let pages = layout_pages(&mut engine, &mut children, locator, styles)?;

    Ok(Document { pages, info, introspector: Introspector::default() })
}

/// Layouts the document's pages.
fn layout_pages<'a>(
    engine: &mut Engine,
    children: &'a mut [Pair<'a>],
    locator: SplitLocator<'a>,
    styles: StyleChain<'a>,
) -> SourceResult<Vec<Page>> {
    // Slice up the children into logical parts.
    let items = collect_page_items(children, locator, styles);

    // Layout the page runs in parallel.
    let mut runs = engine.parallelize(
        items.iter().filter_map(|item| match item {
            PageItem::Run(children, initial, locator) => {
                Some((children, initial, locator.relayout()))
            }
            _ => None,
        }),
        |engine, (children, initial, locator)| {
            layout_page_run(engine, children, locator, *initial)
        },
    );

    let mut pages = vec![];
    let mut tags = vec![];
    let mut counter = ManualPageCounter::new();

    // Collect and finalize the runs, handling things like page parity and tags
    // between pages.
    for item in &items {
        match item {
            PageItem::Run(..) => {
                let layouted = runs.next().unwrap()?;
                for layouted in layouted {
                    let page = finalize_page(engine, &mut counter, &mut tags, layouted)?;
                    pages.push(page);
                }
            }
            PageItem::Parity(parity, initial, locator) => {
                if !parity.matches(pages.len()) {
                    continue;
                }

                let layouted = layout_blank_page(engine, locator.relayout(), *initial)?;
                let page = finalize_page(engine, &mut counter, &mut tags, layouted)?;
                pages.push(page);
            }
            PageItem::Tags(items) => {
                tags.extend(
                    items
                        .iter()
                        .filter_map(|(c, _)| c.to_packed::<TagElem>())
                        .map(|elem| elem.tag.clone()),
                );
            }
        }
    }

    // Add the remaining tags to the very end of the last page.
    if !tags.is_empty() {
        let last = pages.last_mut().unwrap();
        let pos = Point::with_y(last.frame.height());
        last.frame
            .push_multiple(tags.into_iter().map(|tag| (pos, FrameItem::Tag(tag))));
    }

    Ok(pages)
}

/// Slices up the children into logical parts, processing styles and handling
/// things like tags and weak pagebreaks.
fn collect_page_items<'a>(
    mut children: &'a mut [Pair<'a>],
    mut locator: SplitLocator<'a>,
    mut initial: StyleChain<'a>,
) -> Vec<PageItem<'a>> {
    // The collected page-level items.
    let mut items: Vec<PageItem<'a>> = vec![];
    // When this is true, an empty page should be added to `pages` at the end.
    let mut staged_empty_page = true;

    // The `children` are a flat list of flow-level items and pagebreaks. This
    // loops splits it up into pagebreaks and consecutive slices of
    // non-pagebreaks. From these pieces, we build page items that we can then
    // layout in parallel.
    while let Some(&(elem, styles)) = children.first() {
        if let Some(pagebreak) = elem.to_packed::<PagebreakElem>() {
            // Add a blank page if we encounter a strong pagebreak and there was
            // a staged empty page.
            let strong = !pagebreak.weak(styles);
            if strong && staged_empty_page {
                let locator = locator.next(&elem.span());
                items.push(PageItem::Run(&[], initial, locator));
            }

            // Add an instruction to adjust the page parity if requested.
            if let Some(parity) = pagebreak.to(styles) {
                let locator = locator.next(&elem.span());
                items.push(PageItem::Parity(parity, styles, locator));
            }

            // The initial styles for the next page are ours unless this is a
            // "boundary" pagebreak. Such a pagebreak is generated at the end of
            // the scope of a page set rule to ensure a page boundary. It's
            // styles correspond to the styles _before_ the page set rule, so we
            // don't want to apply it to a potential empty page.
            if !pagebreak.boundary(styles) {
                initial = styles;
            }

            // Stage an empty page after a strong pagebreak.
            staged_empty_page |= strong;

            // Advance to the next child.
            children = &mut children[1..];
        } else {
            // Find the end of the consecutive non-pagebreak run.
            let end =
                children.iter().take_while(|(c, _)| !c.is::<PagebreakElem>()).count();

            // Migrate start tags without accompanying end tags from before a
            // pagebreak to after it.
            let end = migrate_unterminated_tags(children, end);
            if end == 0 {
                continue;
            }

            // Advance to the rest of the children.
            let (group, rest) = children.split_at_mut(end);
            children = rest;

            // If all that is left now are tags, then we don't want to add a
            // page just for them (since no group would have been detected in a
            // tagless layout and tags should never affect the layout). For this
            // reason, we remember them in a `PageItem::Tags` and later insert
            // them at the _very start_ of the next page, even before the
            // header.
            //
            // We don't do this if all that's left is end boundary pagebreaks
            // and if an empty page is still staged, since then we can just
            // conceptually replace that final page with us.
            if group.iter().all(|(c, _)| c.is::<TagElem>())
                && !(staged_empty_page
                    && children.iter().all(|&(c, s)| {
                        c.to_packed::<PagebreakElem>().is_some_and(|c| c.boundary(s))
                    }))
            {
                items.push(PageItem::Tags(group));
                continue;
            }

            // Record a page run and then disregard a staged empty page because
            // we have real content now.
            let locator = locator.next(&elem.span());
            items.push(PageItem::Run(group, initial, locator));
            staged_empty_page = false;
        }
    }

    // Flush a staged empty page.
    if staged_empty_page {
        items.push(PageItem::Run(&[], initial, locator.next(&())));
    }

    items
}

/// Migrates trailing start tags without accompanying end tags tags from before
/// a pagebreak to after it. Returns the position right after the last
/// non-migrated tag.
///
/// This is important because we want the positions of introspectible elements
/// that technically started before a pagebreak, but have no visible content
/// yet, to be after the pagebreak. A typical case where this happens is `show
/// heading: it => pagebreak() + it`.
fn migrate_unterminated_tags(children: &mut [Pair], mid: usize) -> usize {
    // Compute the range from before the first trailing tag to after the last
    // following pagebreak.
    let (before, after) = children.split_at(mid);
    let start = mid - before.iter().rev().take_while(|&(c, _)| c.is::<TagElem>()).count();
    let end = mid + after.iter().take_while(|&(c, _)| c.is::<PagebreakElem>()).count();

    // Determine the set of tag locations which we won't migrate (because they
    // are terminated).
    let excluded: HashSet<_> = children[start..mid]
        .iter()
        .filter_map(|(c, _)| c.to_packed::<TagElem>())
        .filter(|elem| elem.tag.kind() == TagKind::End)
        .map(|elem| elem.tag.location())
        .collect();

    // A key function that partitions the area of interest into three groups:
    // Excluded tags (-1) | Pagebreaks (0) | Migrated tags (1).
    let key = |(c, _): &Pair| match c.to_packed::<TagElem>() {
        Some(elem) => {
            if excluded.contains(&elem.tag.location()) {
                -1
            } else {
                1
            }
        }
        None => 0,
    };

    // Partition the children using a *stable* sort. While it would be possible
    // to write a more efficient direct algorithm for this, the sort version is
    // less likely to have bugs and this is absolutely not on a hot path.
    children[start..end].sort_by_key(key);

    // Compute the new end index, right before the pagebreaks.
    start + children[start..end].iter().take_while(|pair| key(pair) == -1).count()
}

/// Layout a page run with uniform properties.
#[typst_macros::time(name = "page run")]
fn layout_page_run(
    engine: &mut Engine,
    children: &[Pair],
    locator: Locator,
    initial: StyleChain,
) -> SourceResult<Vec<LayoutedPage>> {
    layout_page_run_impl(
        engine.world,
        engine.introspector,
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        children,
        locator.track(),
        initial,
    )
}

/// Layout a single page suitable  for parity adjustment.
fn layout_blank_page(
    engine: &mut Engine,
    locator: Locator,
    initial: StyleChain,
) -> SourceResult<LayoutedPage> {
    let layouted = layout_page_run(engine, &[], locator, initial)?;
    Ok(layouted.into_iter().next().unwrap())
}

/// The internal implementation of `layout_page_run`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_page_run_impl(
    world: Tracked<dyn World + '_>,
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    children: &[Pair],
    locator: Tracked<Locator>,
    initial: StyleChain,
) -> SourceResult<Vec<LayoutedPage>> {
    let link = LocatorLink::new(locator);
    let mut locator = Locator::link(&link).split();
    let mut engine = Engine {
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route),
    };

    // Determine the page-wide styles.
    let styles = determine_page_styles(children, initial);
    let styles = StyleChain::new(&styles);
    let span = first_span(children);

    // When one of the lengths is infinite the page fits its content along
    // that axis.
    let width = PageElem::width_in(styles).unwrap_or(Abs::inf());
    let height = PageElem::height_in(styles).unwrap_or(Abs::inf());
    let mut size = Size::new(width, height);
    if PageElem::flipped_in(styles) {
        std::mem::swap(&mut size.x, &mut size.y);
    }

    let mut min = width.min(height);
    if !min.is_finite() {
        min = Paper::A4.width();
    }

    // Determine the margins.
    let default = Rel::<Length>::from((2.5 / 21.0) * min);
    let margin = PageElem::margin_in(styles);
    let two_sided = margin.two_sided.unwrap_or(false);
    let margin = margin
        .sides
        .map(|side| side.and_then(Smart::custom).unwrap_or(default))
        .resolve(styles)
        .relative_to(size);

    // Realize columns.
    let area = size - margin.sum_by_axis();
    let mut regions = Regions::repeat(area, area.map(Abs::is_finite));
    regions.root = true;

    let fill = PageElem::fill_in(styles);
    let foreground = PageElem::foreground_in(styles);
    let background = PageElem::background_in(styles);
    let header_ascent = PageElem::header_ascent_in(styles).relative_to(margin.top);
    let footer_descent = PageElem::footer_descent_in(styles).relative_to(margin.bottom);
    let numbering = PageElem::numbering_in(styles);
    let number_align = PageElem::number_align_in(styles);
    let binding =
        PageElem::binding_in(styles).unwrap_or_else(|| match TextElem::dir_in(styles) {
            Dir::LTR => Binding::Left,
            _ => Binding::Right,
        });

    // Construct the numbering (for header or footer).
    let numbering_marginal = numbering.as_ref().map(|numbering| {
        let both = match numbering {
            Numbering::Pattern(pattern) => pattern.pieces() >= 2,
            Numbering::Func(_) => true,
        };

        let mut counter = CounterDisplayElem::new(
            Counter::new(CounterKey::Page),
            Smart::Custom(numbering.clone()),
            both,
        )
        .pack()
        .spanned(span);

        // We interpret the Y alignment as selecting header or footer
        // and then ignore it for aligning the actual number.
        if let Some(x) = number_align.x() {
            counter = counter.aligned(x.into());
        }

        counter
    });

    let header = PageElem::header_in(styles);
    let footer = PageElem::footer_in(styles);
    let (header, footer) = if matches!(number_align.y(), Some(OuterVAlignment::Top)) {
        (header.as_ref().unwrap_or(&numbering_marginal), footer.as_ref().unwrap_or(&None))
    } else {
        (header.as_ref().unwrap_or(&None), footer.as_ref().unwrap_or(&numbering_marginal))
    };

    // Layout the children.
    let fragment = FlowLayouter::new(
        &mut engine,
        children,
        locator.next(&span).split(),
        styles,
        regions,
        PageElem::columns_in(styles),
        ColumnsElem::gutter_in(styles),
        span,
        &mut vec![],
    )
    .layout(regions)?;

    // Layouts a single marginal.
    let mut layout_marginal = |content: &Option<Content>, area, align| {
        let Some(content) = content else { return Ok(None) };
        let aligned = content.clone().styled(AlignElem::set_alignment(align));
        layout_frame(
            &mut engine,
            &aligned,
            locator.next(&content.span()),
            styles,
            Region::new(area, Axes::splat(true)),
        )
        .map(Some)
    };

    // Layout marginals.
    let mut layouted = Vec::with_capacity(fragment.len());
    for inner in fragment {
        let header_size = Size::new(inner.width(), margin.top - header_ascent);
        let footer_size = Size::new(inner.width(), margin.bottom - footer_descent);
        let full_size = inner.size() + margin.sum_by_axis();
        let mid = HAlignment::Center + VAlignment::Horizon;
        layouted.push(LayoutedPage {
            inner,
            fill: fill.clone(),
            numbering: numbering.clone(),
            header: layout_marginal(header, header_size, Alignment::BOTTOM)?,
            footer: layout_marginal(footer, footer_size, Alignment::TOP)?,
            background: layout_marginal(background, full_size, mid)?,
            foreground: layout_marginal(foreground, full_size, mid)?,
            margin,
            binding,
            two_sided,
        });
    }

    Ok(layouted)
}

/// Determines the styles used for a page run itself and page-level content like
/// marginals and footnotes.
///
/// As a base, we collect the styles that are shared by all elements on the page
/// run. As a fallback if there are no elements, we use the styles active at the
/// pagebreak that introduced the page (at the very start, we use the default
/// styles). Then, to produce our page styles, we filter this list of styles
/// according to a few rules:
///
/// - Other styles are only kept if they are `outside && (initial || liftable)`.
/// - "Outside" means they were not produced within a show rule or that the
///   show rule "broke free" to the page level by emitting page styles.
/// - "Initial" means they were active at the pagebreak that introduced the
///   page. Since these are intuitively already active, they should be kept even
///   if not liftable. (E.g. `text(red, page(..)`) makes the footer red.)
/// - "Liftable" means they can be lifted to the page-level even though they
///   weren't yet active at the very beginning. Set rule styles are liftable as
///   opposed to direct constructor calls:
///   - For `set page(..); set text(red)` the red text is kept even though it
///     comes after the weak pagebreak from set page.
///   - For `set page(..); text(red)[..]` the red isn't kept because the
///     constructor styles are not liftable.
fn determine_page_styles(children: &[Pair], initial: StyleChain) -> Styles {
    // Determine the shared styles (excluding tags).
    let tagless = children.iter().filter(|(c, _)| !c.is::<TagElem>()).map(|&(_, s)| s);
    let base = StyleChain::trunk(tagless).unwrap_or(initial).to_map();

    // Determine the initial styles that are also shared by everything. We can't
    // use `StyleChain::trunk` because it currently doesn't deal with partially
    // shared links (where a subslice matches).
    let trunk_len = initial
        .to_map()
        .as_slice()
        .iter()
        .zip(base.as_slice())
        .take_while(|&(a, b)| a == b)
        .count();

    // Filter the base styles according to our rules.
    base.into_iter()
        .enumerate()
        .filter(|(i, style)| {
            let initial = *i < trunk_len;
            style.outside() && (initial || style.liftable())
        })
        .map(|(_, style)| style)
        .collect()
}

/// Piece together the inner page frame and the marginals. We can only do this
/// at the very end because inside/outside margins require knowledge of the
/// physical page number, which is unknown during parallel layout.
fn finalize_page(
    engine: &mut Engine,
    counter: &mut ManualPageCounter,
    tags: &mut Vec<Tag>,
    LayoutedPage {
        inner,
        mut margin,
        binding,
        two_sided,
        header,
        footer,
        background,
        foreground,
        fill,
        numbering,
    }: LayoutedPage,
) -> SourceResult<Page> {
    // If two sided, left becomes inside and right becomes outside.
    // Thus, for left-bound pages, we want to swap on even pages and
    // for right-bound pages, we want to swap on odd pages.
    if two_sided && binding.swap(counter.physical()) {
        std::mem::swap(&mut margin.left, &mut margin.right);
    }

    // Create a frame for the full page.
    let mut frame = Frame::hard(inner.size() + margin.sum_by_axis());

    // Add tags.
    for tag in tags.drain(..) {
        frame.push(Point::zero(), FrameItem::Tag(tag));
    }

    // Add the "before" marginals. The order in which we push things here is
    // important as it affects the relative ordering of introspectible elements
    // and thus how counters resolve.
    if let Some(background) = background {
        frame.push_frame(Point::zero(), background);
    }
    if let Some(header) = header {
        frame.push_frame(Point::with_x(margin.left), header);
    }

    // Add the inner contents.
    frame.push_frame(Point::new(margin.left, margin.top), inner);

    // Add the "after" marginals.
    if let Some(footer) = footer {
        let y = frame.height() - footer.height();
        frame.push_frame(Point::new(margin.left, y), footer);
    }
    if let Some(foreground) = foreground {
        frame.push_frame(Point::zero(), foreground);
    }

    // Apply counter updates from within the page to the manual page counter.
    counter.visit(engine, &frame)?;

    // Get this page's number and then bump the counter for the next page.
    let number = counter.logical();
    counter.step();

    Ok(Page { frame, fill, numbering, number })
}

/// Layout content into multiple regions.
///
/// When just layouting into a single region, prefer [`layout_frame`].
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
    )
}

/// Layout content into regions with columns.
///
/// For now, this just invokes normal layout on cycled smaller regions. However,
/// in the future, columns will be able to interact (e.g. through floating
/// figures), so this is already factored out because it'll be conceptually
/// different from just layouting into more smaller regions.
pub fn layout_fragment_with_columns(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
    count: NonZeroUsize,
    gutter: Rel<Abs>,
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
    )
}

/// Layout content into a single region.
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
) -> SourceResult<Fragment> {
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

    // If we are in a `PageElem`, this might already be a realized flow.
    let arenas = Arenas::default();
    let children =
        realizer_container(&mut engine, &mut locator, &arenas, content, styles)?;

    FlowLayouter::new(
        &mut engine,
        &children,
        locator,
        styles,
        regions,
        columns,
        column_gutter,
        content.span(),
        &mut vec![],
    )
    .layout(regions)
}

/// Layouts a collection of block-level elements.
struct FlowLayouter<'a, 'e> {
    /// The engine.
    engine: &'a mut Engine<'e>,
    /// The children that will be arranged into a flow.
    children: &'a [Pair<'a>],
    /// A span to use for errors.
    span: Span,
    /// Whether this is the root flow.
    root: bool,
    /// Provides unique locations to the flow's children.
    locator: SplitLocator<'a>,
    /// The shared styles.
    shared: StyleChain<'a>,
    /// The number of columns.
    columns: usize,
    /// The gutter between columns.
    column_gutter: Abs,
    /// The regions to layout children into. These already incorporate the
    /// columns.
    regions: Regions<'a>,
    /// Whether the flow should expand to fill the region.
    expand: Axes<bool>,
    /// The initial size of `regions.size` that was available before we started
    /// subtracting.
    initial: Size,
    /// Whether the last block was a paragraph.
    ///
    /// Used for indenting paragraphs after the first in a block.
    last_was_par: bool,
    /// Spacing and layouted blocks for the current region.
    items: Vec<FlowItem>,
    /// A queue of tags that will be attached to the next frame.
    pending_tags: Vec<&'a Tag>,
    /// A queue of floating elements.
    pending_floats: Vec<FlowItem>,
    /// Whether we have any footnotes in the current region.
    has_footnotes: bool,
    /// Footnote configuration.
    footnote_config: FootnoteConfig,
    /// Footnotes that we have already processed.
    visited_footnotes: HashSet<Location>,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

/// Cached footnote configuration.
struct FootnoteConfig {
    separator: Content,
    clearance: Abs,
    gap: Abs,
}

/// A prepared item in a flow layout.
#[derive(Debug)]
enum FlowItem {
    /// Spacing between other items and whether it is weak.
    Absolute(Abs, bool),
    /// Fractional spacing between other items.
    Fractional(Fr),
    /// A frame for a layouted block.
    Frame {
        /// The frame itself.
        frame: Frame,
        /// How to align the frame.
        align: Axes<FixedAlignment>,
        /// Whether the frame sticks to the item after it (for orphan prevention).
        sticky: bool,
        /// Whether the frame is movable; that is, kept together with its
        /// footnotes.
        ///
        /// This is true for frames created by paragraphs and
        /// [`BlockElem::single_layouter`] elements.
        movable: bool,
    },
    /// An absolutely placed frame.
    Placed {
        /// The layouted content.
        frame: Frame,
        /// Where to place the content horizontally.
        x_align: FixedAlignment,
        /// Where to place the content vertically.
        y_align: Smart<Option<FixedAlignment>>,
        /// A translation to apply to the content.
        delta: Axes<Rel<Abs>>,
        /// Whether the content floats --- i.e. collides with in-flow content.
        float: bool,
        /// The amount of space that needs to be kept between the placed content
        /// and in-flow content. Only relevant if `float` is `true`.
        clearance: Abs,
    },
    /// A footnote frame (can also be the separator).
    Footnote(Frame),
}

impl FlowItem {
    /// Whether this item is out-of-flow.
    ///
    /// Out-of-flow items are guaranteed to have a [zero size][Size::zero()].
    fn is_out_of_flow(&self) -> bool {
        match self {
            Self::Placed { float: false, .. } => true,
            Self::Frame { frame, .. } => {
                frame.size().is_zero()
                    && frame.items().all(|(_, item)| {
                        matches!(item, FrameItem::Link(_, _) | FrameItem::Tag(_))
                    })
            }
            _ => false,
        }
    }
}

impl<'a, 'e> FlowLayouter<'a, 'e> {
    /// Create a new flow layouter.
    #[allow(clippy::too_many_arguments)]
    fn new(
        engine: &'a mut Engine<'e>,
        children: &'a [Pair<'a>],
        locator: SplitLocator<'a>,
        shared: StyleChain<'a>,
        mut regions: Regions<'a>,
        columns: NonZeroUsize,
        column_gutter: Rel<Abs>,
        span: Span,
        backlog: &'a mut Vec<Abs>,
    ) -> Self {
        // Separating the infinite space into infinite columns does not make
        // much sense.
        let mut columns = columns.get();
        if !regions.size.x.is_finite() {
            columns = 1;
        }

        // Determine the width of the gutter and each column.
        let column_gutter = column_gutter.relative_to(regions.base().x);

        if columns > 1 {
            *backlog = std::iter::once(&regions.size.y)
                .chain(regions.backlog)
                .flat_map(|&height| std::iter::repeat(height).take(columns))
                .skip(1)
                .collect();

            let width =
                (regions.size.x - column_gutter * (columns - 1) as f64) / columns as f64;

            // Create the pod regions.
            regions = Regions {
                size: Size::new(width, regions.size.y),
                full: regions.full,
                backlog,
                last: regions.last,
                expand: Axes::new(true, regions.expand.y),
                root: regions.root,
            };
        }

        // Check whether we have just a single multiple-layoutable element. In
        // that case, we do not set `expand.y` to `false`, but rather keep it at
        // its original value (since that element can take the full space).
        //
        // Consider the following code: `block(height: 5cm, pad(10pt,
        // align(bottom, ..)))`. Thanks to the code below, the expansion will be
        // passed all the way through the block & pad and reach the innermost
        // flow, so that things are properly bottom-aligned.
        let mut alone = false;
        if let [(child, _)] = children {
            alone = child.is::<BlockElem>();
        }

        // Disable vertical expansion when there are multiple or not directly
        // layoutable children.
        let expand = regions.expand;
        if !alone {
            regions.expand.y = false;
        }

        // The children aren't root.
        let root = std::mem::replace(&mut regions.root, false);

        Self {
            engine,
            children,
            span,
            root,
            locator,
            shared,
            columns,
            column_gutter,
            regions,
            expand,
            initial: regions.size,
            last_was_par: false,
            items: vec![],
            pending_tags: vec![],
            pending_floats: vec![],
            has_footnotes: false,
            footnote_config: FootnoteConfig {
                separator: FootnoteEntry::separator_in(shared),
                clearance: FootnoteEntry::clearance_in(shared),
                gap: FootnoteEntry::gap_in(shared),
            },
            visited_footnotes: HashSet::new(),
            finished: vec![],
        }
    }

    /// Layout the flow.
    fn layout(mut self, regions: Regions) -> SourceResult<Fragment> {
        for &(child, styles) in self.children {
            if let Some(elem) = child.to_packed::<TagElem>() {
                self.handle_tag(elem);
            } else if let Some(elem) = child.to_packed::<VElem>() {
                self.handle_v(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<ColbreakElem>() {
                self.handle_colbreak(elem)?;
            } else if let Some(elem) = child.to_packed::<ParElem>() {
                self.handle_par(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<BlockElem>() {
                self.handle_block(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<PlaceElem>() {
                self.handle_place(elem, styles)?;
            } else if let Some(elem) = child.to_packed::<FlushElem>() {
                self.handle_flush(elem)?;
            } else {
                bail!(child.span(), "unexpected flow child");
            }
        }

        self.finish(regions)
    }

    /// Place explicit metadata into the flow.
    fn handle_tag(&mut self, elem: &'a Packed<TagElem>) {
        self.pending_tags.push(&elem.tag);
    }

    /// Layout vertical spacing.
    fn handle_v(&mut self, v: &'a Packed<VElem>, styles: StyleChain) -> SourceResult<()> {
        self.handle_item(match v.amount {
            Spacing::Rel(rel) => FlowItem::Absolute(
                // Resolve the spacing relative to the current base height.
                rel.resolve(styles).relative_to(self.initial.y),
                v.weakness(styles) > 0,
            ),
            Spacing::Fr(fr) => FlowItem::Fractional(fr),
        })
    }

    /// Layout a column break.
    fn handle_colbreak(&mut self, _: &'a Packed<ColbreakElem>) -> SourceResult<()> {
        // If there is still an available region, skip to it.
        // TODO: Turn this into a region abstraction.
        if !self.regions.backlog.is_empty() || self.regions.last.is_some() {
            self.finish_region(true)?;
        }
        Ok(())
    }

    /// Layout a paragraph.
    #[typst_macros::time(name = "par", span = par.span())]
    fn handle_par(
        &mut self,
        par: &'a Packed<ParElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Fetch properties.
        let align = AlignElem::alignment_in(styles).resolve(styles);
        let leading = ParElem::leading_in(styles);
        let costs = TextElem::costs_in(styles);

        // Layout the paragraph into lines. This only depends on the base size,
        // not on the Y position.
        let consecutive = self.last_was_par;
        let locator = self.locator.next(&par.span());
        let lines = crate::layout::layout_inline(
            self.engine,
            &par.children,
            locator,
            styles,
            consecutive,
            self.regions.base(),
            self.regions.expand.x,
        )?
        .into_frames();

        // If the first line doesn’t fit in this region, then defer any
        // previous sticky frame to the next region (if available)
        if let Some(first) = lines.first() {
            while !self.regions.size.y.fits(first.height()) && !self.regions.in_last() {
                let in_last = self.finish_region_with_migration()?;
                if in_last {
                    break;
                }
            }
        }

        // Determine whether to prevent widow and orphans.
        let len = lines.len();
        let prevent_orphans =
            costs.orphan() > Ratio::zero() && len >= 2 && !lines[1].is_empty();
        let prevent_widows =
            costs.widow() > Ratio::zero() && len >= 2 && !lines[len - 2].is_empty();
        let prevent_all = len == 3 && prevent_orphans && prevent_widows;

        // Store the heights of lines at the edges because we'll potentially
        // need these later when `lines` is already moved.
        let height_at = |i| lines.get(i).map(Frame::height).unwrap_or_default();
        let front_1 = height_at(0);
        let front_2 = height_at(1);
        let back_2 = height_at(len.saturating_sub(2));
        let back_1 = height_at(len.saturating_sub(1));

        // Layout the lines.
        for (i, mut frame) in lines.into_iter().enumerate() {
            if i > 0 {
                self.handle_item(FlowItem::Absolute(leading, true))?;
            }

            // To prevent widows and orphans, we require enough space for
            // - all lines if it's just three
            // - the first two lines if we're at the first line
            // - the last two lines if we're at the second to last line
            let needed = if prevent_all && i == 0 {
                front_1 + leading + front_2 + leading + back_1
            } else if prevent_orphans && i == 0 {
                front_1 + leading + front_2
            } else if prevent_widows && i >= 2 && i + 2 == len {
                back_2 + leading + back_1
            } else {
                frame.height()
            };

            // If the line(s) don't fit into this region, but they do fit into
            // the next, then advance.
            if !self.regions.in_last()
                && !self.regions.size.y.fits(needed)
                && self.regions.iter().nth(1).is_some_and(|region| region.y.fits(needed))
            {
                self.finish_region(false)?;
            }

            self.drain_tag(&mut frame);
            self.handle_item(FlowItem::Frame {
                frame,
                align,
                sticky: false,
                movable: true,
            })?;
        }

        self.last_was_par = true;
        Ok(())
    }

    /// Layout into multiple regions.
    fn handle_block(
        &mut self,
        block: &'a Packed<BlockElem>,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        // Fetch properties.
        let sticky = block.sticky(styles);
        let align = AlignElem::alignment_in(styles).resolve(styles);

        // If the block is "rootable" it may host footnotes. In that case, we
        // defer rootness to it temporarily. We disable our own rootness to
        // prevent duplicate footnotes.
        let is_root = self.root;
        if is_root && block.rootable(styles) {
            self.root = false;
            self.regions.root = true;
        }

        // Skip directly if region is already full.
        if self.regions.is_full() {
            self.finish_region(false)?;
        }

        // Layout the block itself.
        let fragment = block.layout(
            self.engine,
            self.locator.next(&block.span()),
            styles,
            self.regions,
        )?;

        let mut notes = Vec::new();
        for (i, mut frame) in fragment.into_iter().enumerate() {
            // Find footnotes in the frame.
            if self.root {
                self.collect_footnotes(&mut notes, &frame);
            }

            if i > 0 {
                self.finish_region(false)?;
            }

            self.drain_tag(&mut frame);
            frame.post_process(styles);
            self.handle_item(FlowItem::Frame { frame, align, sticky, movable: false })?;
        }

        self.try_handle_footnotes(notes)?;

        self.root = is_root;
        self.regions.root = false;
        self.last_was_par = false;

        Ok(())
    }

    /// Layout a placed element.
    fn handle_place(
        &mut self,
        placed: &'a Packed<PlaceElem>,
        styles: StyleChain,
    ) -> SourceResult<()> {
        // Fetch properties.
        let float = placed.float(styles);
        let clearance = placed.clearance(styles);
        let alignment = placed.alignment(styles);
        let delta = Axes::new(placed.dx(styles), placed.dy(styles)).resolve(styles);

        let x_align = alignment.map_or(FixedAlignment::Center, |align| {
            align.x().unwrap_or_default().resolve(styles)
        });
        let y_align = alignment.map(|align| align.y().map(|y| y.resolve(styles)));

        let mut frame = placed.layout(
            self.engine,
            self.locator.next(&placed.span()),
            styles,
            self.regions.base(),
        )?;

        frame.post_process(styles);

        self.handle_item(FlowItem::Placed {
            frame,
            x_align,
            y_align,
            delta,
            float,
            clearance,
        })
    }

    /// Lays out all floating elements before continuing with other content.
    fn handle_flush(&mut self, _: &'a Packed<FlushElem>) -> SourceResult<()> {
        for item in std::mem::take(&mut self.pending_floats) {
            self.handle_item(item)?;
        }
        while !self.pending_floats.is_empty() {
            self.finish_region(false)?;
        }
        Ok(())
    }

    /// Layout a finished frame.
    fn handle_item(&mut self, mut item: FlowItem) -> SourceResult<()> {
        match item {
            FlowItem::Absolute(v, weak) => {
                if weak
                    && !self
                        .items
                        .iter()
                        .any(|item| matches!(item, FlowItem::Frame { .. },))
                {
                    return Ok(());
                }
                self.regions.size.y -= v
            }
            FlowItem::Fractional(..) => {}
            FlowItem::Frame { ref frame, movable, .. } => {
                let height = frame.height();
                while !self.regions.size.y.fits(height) && !self.regions.in_last() {
                    self.finish_region(false)?;
                }

                let in_last = self.regions.in_last();
                self.regions.size.y -= height;
                if self.root && movable {
                    let mut notes = Vec::new();
                    self.collect_footnotes(&mut notes, frame);
                    self.items.push(item);

                    // When we are already in_last, we can directly force the
                    // footnotes.
                    if !self.handle_footnotes(&mut notes, true, in_last)? {
                        let item = self.items.pop();
                        self.finish_region(false)?;
                        self.items.extend(item);
                        self.regions.size.y -= height;
                        self.handle_footnotes(&mut notes, true, true)?;
                    }
                    return Ok(());
                }
            }
            FlowItem::Placed { float: false, .. } => {}
            FlowItem::Placed {
                ref mut frame,
                ref mut y_align,
                float: true,
                clearance,
                ..
            } => {
                // If there is a queued float in front or if the float doesn't
                // fit, queue it for the next region.
                if !self.pending_floats.is_empty()
                    || (!self.regions.size.y.fits(frame.height() + clearance)
                        && !self.regions.in_last())
                {
                    self.pending_floats.push(item);
                    return Ok(());
                }

                // Select the closer placement, top or bottom.
                if y_align.is_auto() {
                    let ratio = (self.regions.size.y
                        - (frame.height() + clearance) / 2.0)
                        / self.regions.full;
                    let better_align = if ratio <= 0.5 {
                        FixedAlignment::End
                    } else {
                        FixedAlignment::Start
                    };
                    *y_align = Smart::Custom(Some(better_align));
                }

                // Add some clearance so that the float doesn't touch the main
                // content.
                frame.size_mut().y += clearance;
                if *y_align == Smart::Custom(Some(FixedAlignment::End)) {
                    frame.translate(Point::with_y(clearance));
                }

                self.regions.size.y -= frame.height();

                // Find footnotes in the frame.
                if self.root {
                    let mut notes = vec![];
                    self.collect_footnotes(&mut notes, frame);
                    self.try_handle_footnotes(notes)?;
                }
            }
            FlowItem::Footnote(_) => {}
        }

        self.items.push(item);
        Ok(())
    }

    /// Attach currently pending metadata to the frame.
    fn drain_tag(&mut self, frame: &mut Frame) {
        if !self.pending_tags.is_empty() && !frame.is_empty() {
            frame.prepend_multiple(
                self.pending_tags
                    .drain(..)
                    .map(|tag| (Point::zero(), FrameItem::Tag(tag.clone()))),
            );
        }
    }

    /// Finisht the region, migrating all sticky items to the next one.
    ///
    /// Returns whether we migrated into a last region.
    fn finish_region_with_migration(&mut self) -> SourceResult<bool> {
        // Find the suffix of sticky items.
        let mut sticky = self.items.len();
        for (i, item) in self.items.iter().enumerate().rev() {
            match *item {
                FlowItem::Absolute(_, _) => {}
                FlowItem::Frame { sticky: true, .. } => sticky = i,
                _ => break,
            }
        }

        let carry: Vec<_> = self.items.drain(sticky..).collect();
        self.finish_region(false)?;

        let in_last = self.regions.in_last();
        for item in carry {
            self.handle_item(item)?;
        }

        Ok(in_last)
    }

    /// Finish the frame for one region.
    ///
    /// Set `force` to `true` to allow creating a frame for out-of-flow elements
    /// only (this is used to force the creation of a frame in case the
    /// remaining elements are all out-of-flow).
    fn finish_region(&mut self, force: bool) -> SourceResult<()> {
        // Early return if we don't have any relevant items.
        if !force
            && !self.items.is_empty()
            && self.items.iter().all(FlowItem::is_out_of_flow)
        {
            self.finished.push(Frame::soft(self.initial));
            self.regions.next();
            self.initial = self.regions.size;
            return Ok(());
        }

        // Trim weak spacing.
        while self
            .items
            .last()
            .is_some_and(|item| matches!(item, FlowItem::Absolute(_, true)))
        {
            self.items.pop();
        }

        // Determine the used size.
        let mut fr = Fr::zero();
        let mut used = Size::zero();
        let mut footnote_height = Abs::zero();
        let mut float_top_height = Abs::zero();
        let mut float_bottom_height = Abs::zero();
        let mut first_footnote = true;
        for item in &self.items {
            match item {
                FlowItem::Absolute(v, _) => used.y += *v,
                FlowItem::Fractional(v) => fr += *v,
                FlowItem::Frame { frame, .. } => {
                    used.y += frame.height();
                    used.x.set_max(frame.width());
                }
                FlowItem::Placed { float: false, .. } => {}
                FlowItem::Placed { frame, float: true, y_align, .. } => match y_align {
                    Smart::Custom(Some(FixedAlignment::Start)) => {
                        float_top_height += frame.height()
                    }
                    Smart::Custom(Some(FixedAlignment::End)) => {
                        float_bottom_height += frame.height()
                    }
                    _ => {}
                },
                FlowItem::Footnote(frame) => {
                    footnote_height += frame.height();
                    if !first_footnote {
                        footnote_height += self.footnote_config.gap;
                    }
                    first_footnote = false;
                    used.x.set_max(frame.width());
                }
            }
        }
        used.y += footnote_height + float_top_height + float_bottom_height;

        // Determine the size of the flow in this region depending on whether
        // the region expands. Also account for fractional spacing and
        // footnotes.
        let mut size = self.expand.select(self.initial, used).min(self.initial);
        if (fr.get() > 0.0 || self.has_footnotes) && self.initial.y.is_finite() {
            size.y = self.initial.y;
        }

        if !self.regions.size.x.is_finite() && self.expand.x {
            bail!(self.span, "cannot expand into infinite width");
        }
        if !self.regions.size.y.is_finite() && self.expand.y {
            bail!(self.span, "cannot expand into infinite height");
        }

        let mut output = Frame::soft(size);
        let mut ruler = FixedAlignment::Start;
        let mut float_top_offset = Abs::zero();
        let mut offset = float_top_height;
        let mut float_bottom_offset = Abs::zero();
        let mut footnote_offset = Abs::zero();

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                FlowItem::Absolute(v, _) => {
                    offset += v;
                }
                FlowItem::Fractional(v) => {
                    let remaining = self.initial.y - used.y;
                    let length = v.share(fr, remaining);
                    offset += length;
                }
                FlowItem::Frame { frame, align, .. } => {
                    ruler = ruler.max(align.y);
                    let x = align.x.position(size.x - frame.width());
                    let y = offset + ruler.position(size.y - used.y);
                    let pos = Point::new(x, y);
                    offset += frame.height();
                    output.push_frame(pos, frame);
                }
                FlowItem::Placed { frame, x_align, y_align, delta, float, .. } => {
                    let x = x_align.position(size.x - frame.width());
                    let y = if float {
                        match y_align {
                            Smart::Custom(Some(FixedAlignment::Start)) => {
                                let y = float_top_offset;
                                float_top_offset += frame.height();
                                y
                            }
                            Smart::Custom(Some(FixedAlignment::End)) => {
                                let y = size.y - footnote_height - float_bottom_height
                                    + float_bottom_offset;
                                float_bottom_offset += frame.height();
                                y
                            }
                            _ => unreachable!("float must be y aligned"),
                        }
                    } else {
                        match y_align {
                            Smart::Custom(Some(align)) => {
                                align.position(size.y - frame.height())
                            }
                            _ => offset + ruler.position(size.y - used.y),
                        }
                    };

                    let pos = Point::new(x, y)
                        + delta.zip_map(size, Rel::relative_to).to_point();

                    output.push_frame(pos, frame);
                }
                FlowItem::Footnote(frame) => {
                    let y = size.y - footnote_height + footnote_offset;
                    footnote_offset += frame.height() + self.footnote_config.gap;
                    output.push_frame(Point::with_y(y), frame);
                }
            }
        }

        if force && !self.pending_tags.is_empty() {
            let pos = Point::with_y(offset);
            output.push_multiple(
                self.pending_tags
                    .drain(..)
                    .map(|tag| (pos, FrameItem::Tag(tag.clone()))),
            );
        }

        // Advance to the next region.
        self.finished.push(output);
        self.regions.next();
        self.initial = self.regions.size;
        self.has_footnotes = false;

        // Try to place floats into the next region.
        for item in std::mem::take(&mut self.pending_floats) {
            self.handle_item(item)?;
        }

        Ok(())
    }

    /// Finish layouting and return the resulting fragment.
    fn finish(mut self, regions: Regions) -> SourceResult<Fragment> {
        if self.expand.y {
            while !self.regions.backlog.is_empty() {
                self.finish_region(true)?;
            }
        }

        self.finish_region(true)?;
        while !self.items.is_empty() {
            self.finish_region(true)?;
        }

        if self.columns == 1 {
            return Ok(Fragment::frames(self.finished));
        }

        // Stitch together the column for each region.
        let dir = TextElem::dir_in(self.shared);
        let total = (self.finished.len() as f32 / self.columns as f32).ceil() as usize;

        let mut collected = vec![];
        let mut iter = self.finished.into_iter();
        for region in regions.iter().take(total) {
            // The height should be the parent height if we should expand.
            // Otherwise its the maximum column height for the frame. In that
            // case, the frame is first created with zero height and then
            // resized.
            let height = if regions.expand.y { region.y } else { Abs::zero() };
            let mut output = Frame::hard(Size::new(regions.size.x, height));
            let mut cursor = Abs::zero();

            for _ in 0..self.columns {
                let Some(frame) = iter.next() else { break };
                if !regions.expand.y {
                    output.size_mut().y.set_max(frame.height());
                }

                let width = frame.width();
                let x = if dir == Dir::LTR {
                    cursor
                } else {
                    regions.size.x - cursor - width
                };

                output.push_frame(Point::with_x(x), frame);
                cursor += width + self.column_gutter;
            }

            collected.push(output);
        }

        Ok(Fragment::frames(collected))
    }

    /// Tries to process all footnotes in the frame, placing them
    /// in the next region if they could not be placed in the current
    /// one.
    fn try_handle_footnotes(
        &mut self,
        mut notes: Vec<Packed<FootnoteElem>>,
    ) -> SourceResult<()> {
        // When we are already in_last, we can directly force the
        // footnotes.
        if self.root
            && !self.handle_footnotes(&mut notes, false, self.regions.in_last())?
        {
            self.finish_region(false)?;
            self.handle_footnotes(&mut notes, false, true)?;
        }
        Ok(())
    }

    /// Processes all footnotes in the frame.
    ///
    /// Returns true if the footnote entries fit in the allotted
    /// regions.
    fn handle_footnotes(
        &mut self,
        notes: &mut Vec<Packed<FootnoteElem>>,
        movable: bool,
        force: bool,
    ) -> SourceResult<bool> {
        let prev_notes_len = notes.len();
        let prev_items_len = self.items.len();
        let prev_size = self.regions.size;
        let prev_has_footnotes = self.has_footnotes;

        // Process footnotes one at a time.
        let mut k = 0;
        while k < notes.len() {
            if notes[k].is_ref() {
                k += 1;
                continue;
            }

            if !self.has_footnotes {
                self.layout_footnote_separator()?;
            }

            self.regions.size.y -= self.footnote_config.gap;
            let frames = layout_fragment(
                self.engine,
                &FootnoteEntry::new(notes[k].clone()).pack(),
                Locator::synthesize(notes[k].location().unwrap()),
                self.shared,
                self.regions.with_root(false),
            )?
            .into_frames();

            // If the entries didn't fit, abort (to keep footnote and entry
            // together).
            if !force
                && (k == 0 || movable)
                && frames.first().is_some_and(Frame::is_empty)
            {
                // Undo everything.
                notes.truncate(prev_notes_len);
                self.items.truncate(prev_items_len);
                self.regions.size = prev_size;
                self.has_footnotes = prev_has_footnotes;
                return Ok(false);
            }

            let prev = notes.len();
            for (i, frame) in frames.into_iter().enumerate() {
                self.collect_footnotes(notes, &frame);
                if i > 0 {
                    self.finish_region(false)?;
                    self.layout_footnote_separator()?;
                    self.regions.size.y -= self.footnote_config.gap;
                }
                self.regions.size.y -= frame.height();
                self.items.push(FlowItem::Footnote(frame));
            }

            k += 1;

            // Process the nested notes before dealing with further top-level
            // notes.
            let nested = notes.len() - prev;
            if nested > 0 {
                notes[k..].rotate_right(nested);
            }
        }

        Ok(true)
    }

    /// Layout and save the footnote separator, typically a line.
    fn layout_footnote_separator(&mut self) -> SourceResult<()> {
        let expand = Axes::new(self.regions.expand.x, false);
        let pod = Region::new(self.regions.base(), expand);
        let separator = &self.footnote_config.separator;

        // FIXME: Shouldn't use `root()` here.
        let mut frame =
            layout_frame(self.engine, separator, Locator::root(), self.shared, pod)?;
        frame.size_mut().y += self.footnote_config.clearance;
        frame.translate(Point::with_y(self.footnote_config.clearance));

        self.has_footnotes = true;
        self.regions.size.y -= frame.height();
        self.items.push(FlowItem::Footnote(frame));

        Ok(())
    }

    /// Collect all footnotes in a frame.
    fn collect_footnotes(
        &mut self,
        notes: &mut Vec<Packed<FootnoteElem>>,
        frame: &Frame,
    ) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => self.collect_footnotes(notes, &group.frame),
                FrameItem::Tag(tag) => {
                    let Some(footnote) = tag.elem().to_packed::<FootnoteElem>() else {
                        continue;
                    };
                    if self.visited_footnotes.insert(tag.location()) {
                        notes.push(footnote.clone());
                    }
                }
                _ => {}
            }
        }
    }
}

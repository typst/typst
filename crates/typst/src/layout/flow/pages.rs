use std::collections::HashSet;

use comemo::{Track, Tracked, TrackedMut};

use super::layout_flow;
use crate::diag::SourceResult;
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{Content, NativeElement, Resolve, Smart, StyleChain, Styles};
use crate::introspection::{
    Counter, CounterDisplayElem, CounterKey, Introspector, Locator, LocatorLink,
    ManualPageCounter, SplitLocator, Tag, TagElem, TagKind,
};
use crate::layout::{
    layout_frame, Abs, AlignElem, Alignment, Axes, Binding, ColumnsElem, Dir, Frame,
    FrameItem, HAlignment, Length, OuterVAlignment, Page, PageElem, PagebreakElem, Paper,
    Parity, Point, Region, Regions, Rel, Sides, Size, VAlignment,
};
use crate::model::Numbering;
use crate::realize::Pair;
use crate::syntax::Span;
use crate::text::TextElem;
use crate::utils::Numeric;
use crate::visualize::Paint;
use crate::World;

/// An item in page layout.
enum PageItem<'a> {
    /// A page run containing content. All runs will be layouted in parallel.
    Run(&'a [Pair<'a>], StyleChain<'a>, Locator<'a>),
    /// Tags in between pages. These will be prepended to the first start of
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

/// Layouts the document's pages.
pub fn layout_pages<'a>(
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
        .pack();

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
    let bump = bumpalo::Bump::new();
    let fragment = layout_flow(
        &mut engine,
        &bump,
        children,
        &mut locator,
        styles,
        regions,
        PageElem::columns_in(styles),
        ColumnsElem::gutter_in(styles),
        Span::detached(),
    )?;

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

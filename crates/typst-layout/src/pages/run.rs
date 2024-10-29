use comemo::{Track, Tracked, TrackedMut};
use typst_library::diag::SourceResult;
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{
    Content, NativeElement, Resolve, Smart, StyleChain, Styles,
};
use typst_library::introspection::{
    Counter, CounterDisplayElem, CounterKey, Introspector, Locator, LocatorLink, TagElem,
};
use typst_library::layout::{
    Abs, AlignElem, Alignment, Axes, Binding, ColumnsElem, Dir, Frame, HAlignment,
    Length, OuterVAlignment, PageElem, Paper, Region, Regions, Rel, Sides, Size,
    VAlignment,
};
use typst_library::model::Numbering;
use typst_library::routines::{Pair, Routines};
use typst_library::text::TextElem;
use typst_library::visualize::Paint;
use typst_library::World;
use typst_utils::Numeric;

use crate::flow::layout_flow;

/// A mostly finished layout for one page. Needs only knowledge of its exact
/// page number to be finalized into a `Page`. (Because the margins can depend
/// on the page number.)
#[derive(Clone)]
pub struct LayoutedPage {
    pub inner: Frame,
    pub margin: Sides<Abs>,
    pub binding: Binding,
    pub two_sided: bool,
    pub header: Option<Frame>,
    pub footer: Option<Frame>,
    pub background: Option<Frame>,
    pub foreground: Option<Frame>,
    pub fill: Smart<Option<Paint>>,
    pub numbering: Option<Numbering>,
}

/// Layout a single page suitable  for parity adjustment.
pub fn layout_blank_page(
    engine: &mut Engine,
    locator: Locator,
    initial: StyleChain,
) -> SourceResult<LayoutedPage> {
    let layouted = layout_page_run(engine, &[], locator, initial)?;
    Ok(layouted.into_iter().next().unwrap())
}

/// Layout a page run with uniform properties.
#[typst_macros::time(name = "page run")]
pub fn layout_page_run(
    engine: &mut Engine,
    children: &[Pair],
    locator: Locator,
    initial: StyleChain,
) -> SourceResult<Vec<LayoutedPage>> {
    layout_page_run_impl(
        engine.routines,
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

/// The internal implementation of `layout_page_run`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn layout_page_run_impl(
    routines: &Routines,
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
        routines,
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
    let area = size - margin.sum_by_axis();
    let fragment = layout_flow(
        &mut engine,
        children,
        &mut locator,
        styles,
        Regions::repeat(area, area.map(Abs::is_finite)),
        PageElem::columns_in(styles),
        ColumnsElem::gutter_in(styles),
        true,
    )?;

    // Layouts a single marginal.
    let mut layout_marginal = |content: &Option<Content>, area, align| {
        let Some(content) = content else { return Ok(None) };
        let aligned = content.clone().styled(AlignElem::set_alignment(align));
        crate::layout_frame(
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

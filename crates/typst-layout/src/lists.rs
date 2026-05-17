use comemo::Track;
use smallvec::smallvec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Context, Depth, Packed, Resolve, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, Dir, Fragment, Frame, FrameItem, Length, Point, Region, Regions, Size,
};
use typst_library::model::{EnumElem, ListElem, Numbering, ParElem, ParbreakElem};
use typst_library::pdf::PdfMarkerTag;
use typst_library::text::TextElem;
use typst_syntax::Span;

use crate::stack::{StackLayoutChild, layout_stack_internal};

/// Layout the list.
#[typst_macros::time(span = elem.span())]
pub fn layout_list(
    elem: &Packed<ListElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let indent = elem.indent.get(styles);
    let body_indent = elem.body_indent.get(styles);
    let tight = elem.tight.get(styles);
    let gutter = elem.spacing.get(styles).unwrap_or_else(|| {
        if tight { styles.get(ParElem::leading) } else { styles.get(ParElem::spacing) }
    });
    let is_rtl = styles.get(TextElem::dir).resolve(styles) == Dir::RTL;

    let Depth(depth) = styles.get(ListElem::depth);

    // Use the user's preferred vertical alignment. Among other things, it
    // avoids '#set align' interference with the list.
    let marker_align = elem.marker_align.get(styles);
    let baseline_align = marker_align.y().is_none();
    let marker = elem
        .marker
        .get_ref(styles)
        .resolve(engine, styles, depth)?
        .aligned(marker_align);

    let mut items = vec![];
    for item in &elem.children {
        // Text in wide lists shall always turn into paragraphs.
        let mut body = item.body.clone();
        if !tight {
            body += ParbreakElem::shared();
        }
        let body = body.set(ListElem::depth, Depth(1));

        let item = ItemContent {
            marker: PdfMarkerTag::ListItemLabel(marker.clone()),
            body: PdfMarkerTag::ListItemBody(body),
        };

        items.push(item);
    }

    let layouter = ItemsLayouter {
        gutter,
        span: elem.span(),
        indent,
        body_indent,
        baseline_align,
        is_rtl,

        // These will be calculated later.
        marker_width: Abs::zero(),
        body_width: None,
    };

    layout_items(layouter, items, engine, locator, styles, regions)
}

/// Layout the enumeration.
#[typst_macros::time(span = elem.span())]
pub fn layout_enum(
    elem: &Packed<EnumElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let numbering = elem.numbering.get_ref(styles);
    let reversed = elem.reversed.get(styles);
    let indent = elem.indent.get(styles);
    let body_indent = elem.body_indent.get(styles);
    let tight = elem.tight.get(styles);
    let gutter = elem.spacing.get(styles).unwrap_or_else(|| {
        if tight { styles.get(ParElem::leading) } else { styles.get(ParElem::spacing) }
    });
    let is_rtl = styles.get(TextElem::dir).resolve(styles) == Dir::RTL;

    let mut items = vec![];
    let mut number = elem
        .start
        .get(styles)
        .unwrap_or_else(|| if reversed { elem.children.len() as u64 } else { 1 });
    let mut parents = styles.get_cloned(EnumElem::parents);

    let full = elem.full.get(styles);

    // Horizontally align based on the given respective parameter.
    // Vertically align to the top to avoid inheriting `horizon` or `bottom`
    // alignment from the context and having the number be displaced in
    // relation to the item it refers to.
    let number_align = elem.number_align.get(styles);
    let baseline_align = number_align.y().is_none();

    for item in &elem.children {
        number = item.number.get(styles).unwrap_or(number);

        let context = Context::new(None, Some(styles));
        let resolved = if full {
            parents.push(number);
            let content = numbering
                .apply(engine, context.track(), item.span(), &parents)?
                .display();
            parents.pop();
            content
        } else {
            match numbering {
                Numbering::Pattern(pattern) => TextElem::packed(pattern.apply_kth(
                    engine,
                    item.span(),
                    parents.len(),
                    number,
                )),
                other => other
                    .apply(engine, context.track(), item.span(), &[number])?
                    .display(),
            }
        };

        // Disable overhang as a workaround to end-aligned dots glitching
        // and decreasing spacing between numbers and items.
        let resolved = resolved.aligned(number_align).set(TextElem::overhang, false);

        // Text in wide enums shall always turn into paragraphs.
        let mut body = item.body.clone();
        if !tight {
            body += ParbreakElem::shared();
        }

        let body = body.set(EnumElem::parents, smallvec![number]);

        let item = ItemContent {
            marker: PdfMarkerTag::ListItemLabel(resolved),
            body: PdfMarkerTag::ListItemBody(body),
        };

        items.push(item);
        number =
            if reversed { number.saturating_sub(1) } else { number.saturating_add(1) };
    }

    let layouter = ItemsLayouter {
        gutter,
        span: elem.span(),
        indent,
        body_indent,
        baseline_align,
        is_rtl,

        // These will be calculated later.
        marker_width: Abs::zero(),
        body_width: None,
    };

    layout_items(layouter, items, engine, locator, styles, regions)
}

/// Structure with the content for each list item.
struct ItemContent {
    /// The item marker.
    marker: Content,
    /// The item body.
    body: Content,
}

/// Layout data shared across all list items.
struct ItemsLayouter {
    /// Gutter spacing between items.
    gutter: Length,
    /// Span of the list/enum element.
    span: Span,
    /// List indent from the text start.
    indent: Length,
    /// Indent between the marker and the body.
    body_indent: Length,
    /// Whether baseline alignment should be enabled. When disabled, markers
    /// control their own alignment.
    baseline_align: bool,
    /// Whether RTL was the chosen text direction.
    is_rtl: bool,
    /// Maximum measured width of a marker, so they may align horizontally
    /// relative to each other.
    marker_width: Abs,
    /// If the list may not expand to fill the whole region (e.g. if `width:
    /// auto` was used), then this holds the measured width of an item body, so
    /// they may align horizontally relative to each other.
    body_width: Option<Abs>,
}

/// Layout list items.
///
/// This is done in 3 steps:
///
/// 1. Compute marker widths for horizontal marker alignment;
/// 2. If necessary (when within a `width: auto` block or page), compute body
/// widths for horizontal body alignment to work;
/// 3. Generate list item layouters, each of which is responsible for vertical
/// marker alignment (baseline-aligned or not, depending on user settings);
/// 4. Pass each list item to the stack layouter, making it possible for them to
/// expand to the full available width, allowing for center alignment within the
/// item body.
#[typst_macros::time(span = layouter.span)]
fn layout_items(
    mut layouter: ItemsLayouter,
    items: Vec<ItemContent>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let mut locator = locator.split();

    // Measure markers, so we can align them horizontally relative to the
    // largest width.
    let mut marker_width = Abs::zero();

    // Store locators used during measuring to ensure the same locators will be
    // used later when laying out. This is needed to make introspection work
    // properly.
    let mut locators = Vec::with_capacity(items.len());
    for item in &items {
        let marker_locator = locator.next(&item.marker.span());
        let body_locator = locator.next(&item.body.span());
        let marker = crate::layout_frame(
            engine,
            &item.marker,
            marker_locator.relayout(),
            styles,
            Region::new(Axes::new(regions.size.x, Abs::inf()), Axes::splat(false)),
        )?;

        locators.push((marker_locator, body_locator));
        marker_width.set_max(marker.width());
    }

    layouter.marker_width = marker_width;

    if regions.size.x.to_raw().is_infinite() || !regions.expand.x {
        // Infinite space or `width: auto` used. Both would prevent the list
        // from expanding to fit, breaking alignment. Therefore, restrict the
        // list size to the size of the largest item, prompting list items to
        // align between themselves instead of relative to the full page width.
        let mut measured_body_width = Abs::zero();
        for (item, (_, body_locator)) in items.iter().zip(&locators) {
            let body = crate::layout_frame(
                engine,
                &item.body,
                body_locator.relayout(),
                styles,
                Region::new(Axes::new(regions.size.x, Abs::inf()), Axes::splat(false)),
            )?;

            measured_body_width.set_max(body.width());
        }

        layouter.body_width = Some(measured_body_width);
    } else {
        // Let the list body expand to the full width of the environment.
        layouter.body_width = None;
    }

    let cells =
        items
            .iter()
            .zip(&locators)
            .map(|(item, (marker_locator, body_locator))| {
                StackLayoutChild::CustomLayouter(|engine, styles, regions| {
                    layout_item(
                        item,
                        &layouter,
                        engine,
                        marker_locator,
                        body_locator,
                        styles,
                        regions,
                    )
                })
            });

    layout_stack_internal(
        cells,
        layouter.span,
        Some(layouter.gutter.into()),
        Dir::TTB,
        engine,
        // This locator should not be used by cells.
        locator.next(&()),
        styles,
        regions,
    )
}

/// Layout the item.
///
/// Marker and body width should be determined relative to other items, being
/// equivalent to the largest width, relative to which the marker and body
/// should horizontally align. Note that body width is `None` when the region
/// has a fixed width, as then the list will expand to fill it.
#[typst_macros::time(span = item.body.span())]
fn layout_item(
    item: &ItemContent,
    layouter: &ItemsLayouter,
    engine: &mut Engine,
    marker_locator: &Locator,
    body_locator: &Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let indent = layouter.indent.resolve(styles);
    let body_indent = layouter.body_indent.resolve(styles);
    let mut marker = crate::layout_frame(
        engine,
        &item.marker,
        marker_locator.relayout(),
        styles,
        Region::new(
            Axes::new(layouter.marker_width, regions.base().y),
            Axes::new(true, false),
        ),
    )?;
    let marker_size = marker.size();
    let mut fragment = {
        let mut regions = regions;
        if let Some(body_width) = layouter.body_width {
            regions.size.x = body_width;
            regions.expand.x = true;
        } else {
            regions.size.x -= indent + body_indent + marker_size.x;
        }
        crate::layout_fragment(
            engine,
            &item.body,
            body_locator.relayout(),
            styles,
            regions,
        )?
    };

    // First non-empty frame (ignores frames with only tags due to a forced
    // region break).
    let mut first_frame = if should_skip_first_frame(&fragment) { 1 } else { 0 };

    // Difference between marker and body baselines, for alignment. A positive
    // diff means that the marker is above and must move down, whereas a
    // negative diff means that the marker is below, so the body must be moved
    // down instead.
    let diff;

    if layouter.baseline_align {
        diff = if marker.has_baseline()
            && let Some(first) = fragment.as_slice().get(first_frame)
            && first.has_baseline()
        {
            first.baseline() - marker.baseline()
        } else {
            // One of the frames has no natural baseline, so baseline alignment is disabled.
            Abs::zero()
        };
    } else {
        // Explicit marker alignment was chosen, so re-layout the marker with
        // the same height as the body's first frame so it may align itself
        // vertically with the body.
        let mut regions = regions;
        if let Some(first) = fragment.as_slice().get(first_frame) {
            regions.size.y = first.height();
            regions.full = first.height();
        };

        marker = crate::layout_frame(
            engine,
            &item.marker,
            marker_locator.relayout(),
            styles,
            Region::new(
                Axes::new(layouter.marker_width, regions.base().y),
                Axes::splat(true),
            ),
        )?;

        // No baseline alignment whatsoever.
        diff = Abs::zero();
    };

    let (marker_dy, body_dy) = if diff >= Abs::zero() {
        // Marker's baseline is above the body's baseline, so we can align them
        // by moving the baseline downwards.
        (diff, Abs::zero())
    } else {
        // Marker's baseline is below the body's baseline, so move the body
        // down to avoid overflowing the marker into something above the list
        // (item).
        //
        // To do this, we layout the body again but with '-diff' less space, and
        // then move the result '-diff' units downwards. Of course, this could
        // theoretically generate a new result that is even worse - but there is
        // only so much we can do with a finite number of iterations.
        let mut regions = regions;
        regions.size.y += diff;
        if let Some(body_width) = layouter.body_width {
            regions.size.x = body_width;
            regions.expand.x = true;
        } else {
            regions.size.x -= indent + body_indent + marker_size.x;
        }
        fragment = crate::layout_fragment(
            engine,
            &item.body,
            body_locator.relayout(),
            styles,
            regions,
        )?;
        first_frame = if should_skip_first_frame(&fragment) { 1 } else { 0 };

        (Abs::zero(), -diff)
    };

    // Collect the item's frames. Here, we add the marker to the first non-empty
    // frame, and additionally indent the whole body so it appears after the
    // marker.
    let mut frames = vec![];
    for (i, body_frame) in fragment.into_iter().enumerate() {
        let width = indent + body_indent + marker_size.x + body_frame.width();
        let mut frame = Frame::soft(Size::new(
            width,
            (marker_size.y + marker_dy).max(body_frame.height() + body_dy),
        ));

        // Indent the body after the marker.
        let mut body_pos = Point::new(indent + marker_size.x + body_indent, body_dy);

        if layouter.is_rtl {
            // In RTL cells expand to the left, thus the position must
            // additionally be offset by the cell's width.
            body_pos.x = width - (body_pos.x + body_frame.width());
        }

        // Only place the marker on the first non-empty frame.
        if i == first_frame {
            let mut marker_pos = Point::new(indent, marker_dy);
            if layouter.is_rtl {
                marker_pos.x = width - (marker_pos.x + marker_size.x);
            }
            frame.push_frame(marker_pos, marker.clone());
        }

        frame.push_frame(body_pos, body_frame);
        frames.push(frame);
    }

    Ok(Fragment::frames(frames))
}

/// Check whether the first frame is essentially empty (only contains tags).
/// This usually indicates a forced region break, which we should ignore.
fn should_skip_first_frame(fragment: &Fragment) -> bool {
    fragment.len() > 1
        && is_empty_frame(&fragment.as_slice()[0])
        && fragment.iter().skip(1).any(|f| !is_empty_frame(f))
}

/// Check if a frame is empty (taken from grid layouting).
///
/// HACK: Also consider frames empty if they only contain tags. Table
/// and grid cells need to be locatable for pdf accessibility, but
/// the introspection tags interfere with the layouting.
fn is_empty_frame(frame: &Frame) -> bool {
    frame.items().all(|(_, item)| matches!(item, FrameItem::Tag(_)))
}

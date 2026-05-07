use comemo::Track;
use smallvec::smallvec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, Context, Depth, NativeElement, Packed, Resolve, StyleChain,
};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, BlockElem, Dir, Fragment, Frame, FrameItem, Length, Point, Region,
    Regions, Size, StackChild, StackElem,
};
use typst_library::model::{EnumElem, ListElem, Numbering, ParElem, ParbreakElem};
use typst_library::pdf::PdfMarkerTag;
use typst_library::text::TextElem;
use typst_macros::elem;
use typst_syntax::Span;

use crate::stack::layout_stack;

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

        let item = ItemData::new(
            indent,
            body_indent,
            PdfMarkerTag::ListItemLabel(marker.clone()),
            PdfMarkerTag::ListItemBody(body),
            Length::zero(),
            baseline_align,
            is_rtl,
        );
        items.push(item);
    }

    layout_items(items, gutter, elem.span(), engine, locator, styles, regions)
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

        let item = ItemData::new(
            indent,
            body_indent,
            PdfMarkerTag::ListItemLabel(resolved),
            PdfMarkerTag::ListItemBody(body),
            Length::zero(),
            baseline_align,
            is_rtl,
        );
        items.push(item);
        number =
            if reversed { number.saturating_sub(1) } else { number.saturating_add(1) };
    }

    layout_items(items, gutter, elem.span(), engine, locator, styles, regions)
}

/// Layout list items.
///
/// This is done in 3 steps:
///
/// 1. Compute marker widths for horizontal marker alignment;
/// 2. Generate list item layouters, each of which is responsible for vertical
/// marker alignment (baseline-aligned or not, depending on user settings);
/// 3. Pass each list item to the stack layouter, ensuring they expand to the
/// full available width, allowing for center alignment within the item body.
#[typst_macros::time(span = span)]
fn layout_items(
    items: Vec<ItemData>,
    gutter: Length,
    span: Span,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    // Measure markers, so we can align them horizontally relative to the
    // largest width.
    let mut locator = locator.split();
    let mut marker_size = Abs::zero();
    for item in &items {
        let marker = crate::layout_frame(
            engine,
            &item.marker,
            locator.next(&item.marker.span()),
            styles,
            Region::new(Axes::new(regions.size.x, Abs::inf()), Axes::splat(false)),
        )?;

        marker_size.set_max(marker.width());
    }

    let cells = items
        .into_iter()
        .map(|mut elem| {
            elem.marker_size = Length::from(marker_size);
            StackChild::Block(
                BlockElem::multi_layouter(Packed::new(elem), layout_item).pack(),
            )
        })
        .collect();

    let stack = StackElem::new(cells)
        .with_spacing(Some(gutter.into()))
        .with_dir(typst_library::layout::Dir::TTB);

    layout_stack(&Packed::new(stack), engine, locator.next(&()), styles, regions)
}

/// Structure with list item information. This should never be placed in practice.
/// This is only an element (thus imposing restrictions on the accepted field
/// types) so we can store this data within the `stack` children used to layout
/// the list.
#[elem]
struct ItemData {
    /// List indent from the text start.
    #[required]
    indent: Length,
    /// Indent between the marker and the body.
    #[required]
    body_indent: Length,
    /// The item marker.
    #[required]
    marker: Content,
    /// The item body.
    #[required]
    body: Content,
    /// The width to give to the marker. This is the max width of all markers,
    /// so they may align horizontally properly.
    #[required]
    marker_size: Length,
    /// Whether baseline alignment should be enabled. When disabled, markers
    /// control their own alignment.
    #[required]
    baseline_align: bool,
    /// Whether RTL was the chosen text direction.
    #[required]
    is_rtl: bool,
}

/// Layout the item.
#[typst_macros::time(span = item.span())]
fn layout_item(
    item: &Packed<ItemData>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    // Should only be absolute (cannot use Abs due to element definition
    // restrictions).
    debug_assert!(item.marker_size.em.get() == 0.0);

    let mut locator = locator.split();
    let indent = item.indent.resolve(styles);
    let body_indent = item.body_indent.resolve(styles);
    let mut marker = crate::layout_frame(
        engine,
        &item.marker,
        locator.next(&item.marker.span()),
        styles,
        Region::new(
            Axes::new(item.marker_size.abs, regions.base().y),
            Axes::new(true, false),
        ),
    )?;
    let marker_size = marker.size();
    let mut fragment = {
        let mut regions = regions;
        regions.size.x -= indent + body_indent + marker_size.x;
        crate::layout_fragment(
            engine,
            &item.body,
            locator.next(&item.body.span()),
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

    if item.baseline_align {
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
            locator.next(&item.marker.span()),
            styles,
            Region::new(
                Axes::new(item.marker_size.abs, regions.base().y),
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
        regions.size.x -= indent + body_indent + marker_size.x;
        regions.size.y += diff;
        fragment = crate::layout_fragment(
            engine,
            &item.body,
            locator.next(&item.body.span()),
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

        if item.is_rtl {
            // In RTL cells expand to the left, thus the position must
            // additionally be offset by the cell's width.
            body_pos.x = width - (body_pos.x + body_frame.width());
        }

        // Only place the marker on the first non-empty frame.
        if i == first_frame {
            let mut marker_pos = Point::new(indent, marker_dy);
            if item.is_rtl {
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

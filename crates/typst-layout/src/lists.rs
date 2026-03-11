use comemo::Track;
use smallvec::smallvec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, Context, Depth, NativeElement, Packed, Resolve, StyleChain,
};
use typst_library::introspection::Locator;
use typst_library::layout::{
    Abs, Axes, BlockElem, Dir, Fragment, Frame, FrameItem, HAlignment, Length, Point,
    Region, Regions, Size, StackChild, StackElem, VAlignment,
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
    let marker = elem
        .marker
        .get_ref(styles)
        .resolve(engine, styles, depth)?
        // avoid '#set align' interference with the list
        .aligned(HAlignment::Start + VAlignment::Top);

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
            true,
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

    for item in &elem.children {
        number = item.number.get(styles).unwrap_or(number);

        let context = Context::new(None, Some(styles));
        let resolved = if full {
            parents.push(number);
            let content = numbering.apply(engine, context.track(), &parents)?.display();
            parents.pop();
            content
        } else {
            match numbering {
                Numbering::Pattern(pattern) => {
                    TextElem::packed(pattern.apply_kth(parents.len(), number))
                }
                other => other.apply(engine, context.track(), &[number])?.display(),
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
            number_align.y().is_none(),
            is_rtl,
        );
        items.push(item);
        number =
            if reversed { number.saturating_sub(1) } else { number.saturating_add(1) };
    }

    layout_items(items, gutter, elem.span(), engine, locator, styles, regions)
}

/// Layout items.
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
    // 1. Measure markers
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

    // TODO: is this locator invocation right?
    layout_stack(&Packed::new(stack), engine, locator.next(&()), styles, regions)
}

/// Structure with list item information. This should never be placed in practice.
#[elem]
struct ItemData {
    /// List indent from the text start.
    #[required]
    indent: Length,
    /// Body indent from the marker.
    #[required]
    body_indent: Length,
    /// The marker.
    #[required]
    marker: Content,
    /// The body.
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
#[typst_macros::time(span = elem.span())]
fn layout_item(
    elem: &Packed<ItemData>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let mut locator = locator.split();
    // Should only be absolute (cannot use Abs due to element definition
    // restrictions).
    assert!(elem.marker_size.em.get() == 0.0);
    let mut marker = crate::layout_frame(
        engine,
        &elem.marker,
        locator.next(&elem.marker.span()),
        styles,
        Region::new(
            Axes::new(elem.marker_size.abs, regions.base().y),
            Axes::new(true, false),
        ),
    )?;
    let indent = elem.indent.resolve(styles);
    let body_indent = elem.body_indent.resolve(styles);
    let marker_size = marker.size();
    let mut fragment = {
        let mut regions = regions;
        regions.size.x -= indent + body_indent + marker_size.x;
        crate::layout_fragment(
            engine,
            &elem.body,
            locator.next(&elem.body.span()),
            styles,
            regions,
        )?
    };

    let mut skip_first = should_skip_first_frame(&fragment);

    let diff = if elem.baseline_align {
        let baseline = match fragment.as_slice().get(if skip_first { 1 } else { 0 }) {
            Some(first) => extract_baseline(first, Abs::zero()),
            _ => None,
        };

        let marker_baseline = extract_baseline(&marker, Abs::zero());

        match (baseline, marker_baseline) {
            (Some(baseline), Some(marker_baseline)) => baseline - marker_baseline,
            _ => Abs::zero(),
        }
    } else {
        // Re-layout the marker with the same height as the body's first frame
        // so it may align itself vertically.
        let mut regions = regions;
        if let Some(first) = fragment.as_slice().get(if skip_first { 1 } else { 0 }) {
            regions.size.y = first.height();
            regions.full = first.height();
        };

        marker = crate::layout_frame(
            engine,
            &elem.marker,
            locator.next(&elem.marker.span()),
            styles,
            Region::new(
                Axes::new(elem.marker_size.abs, regions.base().y),
                Axes::splat(true),
            ),
        )?;

        Abs::zero()
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
            &elem.body,
            locator.next(&elem.body.span()),
            styles,
            regions,
        )?;
        skip_first = should_skip_first_frame(&fragment);

        (Abs::zero(), -diff)
    };

    let mut frames = vec![];
    for (i, body_frame) in fragment.into_iter().enumerate() {
        let width = indent + body_indent + marker_size.x + body_frame.width();
        let mut frame = Frame::soft(Size::new(
            width,
            (marker_size.y + marker_dy).max(body_frame.height() + body_dy),
        ));

        // Don't place extraneous markers after a region skip.
        if i > 0 || !skip_first {
            let mut marker_pos = Point::new(indent, marker_dy);
            let mut body_pos = Point::new(indent + marker_size.x + body_indent, body_dy);

            if elem.is_rtl {
                // In RTL cells expand to the left, thus the position must
                // additionally be offset by the cell's width.
                marker_pos.x = width - (marker_pos.x + marker_size.x);
                body_pos.x = width - (body_pos.x + body_frame.width());
            }

            frame.push_frame(marker_pos, marker.clone());
            frame.push_frame(body_pos, body_frame);
        }
        frames.push(frame);
    }

    Ok(Fragment::frames(frames))
}

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

fn extract_baseline(first: &Frame, y_offset: Abs) -> Option<Abs> {
    if first.has_baseline() {
        return Some(first.baseline() + y_offset);
    }

    let mut baseline = None;
    for (pos, item) in first.items() {
        let height = pos.y + y_offset;
        let new_baseline = match item {
            FrameItem::Group(group) => extract_baseline(&group.frame, height),
            FrameItem::Text(_) => Some(height),

            // Skip frame items that are only markers, not visible at all.
            FrameItem::Tag(_) | FrameItem::Link(..) => continue,

            // This kind of frame item does not demand baseline alignment.
            FrameItem::Shape(..) | FrameItem::Image(..) => None,
        };

        if let Some(new_baseline) = new_baseline {
            baseline.get_or_insert(Abs::inf()).set_min(new_baseline);
        }
        break;
    }

    baseline
}

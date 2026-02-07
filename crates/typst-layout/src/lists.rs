use comemo::Track;
use smallvec::smallvec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, Context, Depth, NativeElement, Packed, Resolve, StyleChain,
};
use typst_library::introspection::Locator;
use typst_library::layout::grid::resolve::{Cell, CellGrid};
use typst_library::layout::{
    Abs, Axes, BlockElem, Fragment, Frame, HAlignment, Length, Point, Region, Regions,
    Size, Sizing, StackChild, StackElem, VAlignment,
};
use typst_library::model::{EnumElem, ListElem, Numbering, ParElem, ParbreakElem};
use typst_library::pdf::PdfMarkerTag;
use typst_library::text::TextElem;
use typst_macros::elem;

use crate::grid::GridLayouter;
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

    let Depth(depth) = styles.get(ListElem::depth);
    let marker = elem
        .marker
        .get_ref(styles)
        .resolve(engine, styles, depth)?
        // avoid '#set align' interference with the list
        .aligned(HAlignment::Start + VAlignment::Top);

    let mut cells = vec![];
    for item in &elem.children {
        // Text in wide lists shall always turn into paragraphs.
        let mut body = item.body.clone();
        if !tight {
            body += ParbreakElem::shared();
        }
        let body = body.set(ListElem::depth, Depth(1));

        let elem = ItemData::new(
            indent,
            body_indent,
            PdfMarkerTag::ListItemLabel(marker.clone()),
            PdfMarkerTag::ListItemBody(body),
        );
        let item = BlockElem::multi_layouter(Packed::new(elem), layout_item).pack();
        cells.push(StackChild::Block(item));
    }

    let stack = StackElem::new(cells)
        .with_spacing(Some(gutter.into()))
        .with_dir(typst_library::layout::Dir::TTB);

    layout_stack(&Packed::new(stack), engine, locator, styles, regions)
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

    let mut cells = vec![];
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

        let elem = ItemData::new(
            indent,
            body_indent,
            PdfMarkerTag::ListItemLabel(resolved),
            PdfMarkerTag::ListItemBody(body),
        );
        let item = BlockElem::multi_layouter(Packed::new(elem), layout_item).pack();
        cells.push(StackChild::Block(item));
        number =
            if reversed { number.saturating_sub(1) } else { number.saturating_add(1) };
    }

    let stack = StackElem::new(cells)
        .with_spacing(Some(gutter.into()))
        .with_dir(typst_library::layout::Dir::TTB);

    layout_stack(&Packed::new(stack), engine, locator, styles, regions)
}

#[elem]
struct ItemData {
    #[required]
    indent: Length,

    #[required]
    body_indent: Length,

    #[required]
    marker: Content,

    #[required]
    body: Content,
}

/// Layout the item.
#[typst_macros::time(span = elem.span())]
fn layout_item(
    elem: &Packed<ItemData>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    mut regions: Regions,
) -> SourceResult<Fragment> {
    let mut locator = locator.split();
    let mut marker = crate::layout_frame(
        engine,
        &elem.marker,
        locator.next(&elem.marker.span()),
        styles,
        Region::new(regions.size, Axes::splat(false)),
    )?;
    let indent = elem.indent.resolve(styles);
    let body_indent = elem.body_indent.resolve(styles);
    let marker_size = marker.size();
    regions.size.x -= indent + body_indent + marker_size.x;
    let fragment = crate::layout_fragment(
        engine,
        &elem.body,
        locator.next(&elem.body.span()),
        styles,
        regions,
    )?;
    let baseline = fragment
        .as_slice()
        .first()
        .filter(|x| x.has_baseline())
        .map_or(Abs::zero(), |x| x.baseline());

    let mut diff = baseline;
    if marker.has_baseline() {
        diff -= marker.baseline();
    }
    marker.set_baseline(baseline);

    let mut frames = vec![];
    for body_frame in fragment {
        let mut frame = Frame::soft(Size::new(
            indent + body_indent + marker_size.x + body_frame.width(),
            (marker_size.y + diff).max(body_frame.height()),
        ));
        frame.push_frame(Point::new(indent, diff), marker.clone());
        frame.push_frame(Point::with_x(indent + marker_size.x + body_indent), body_frame);
        frames.push(frame);
    }

    Ok(Fragment::frames(frames))
}

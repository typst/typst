use comemo::Track;
use smallvec::smallvec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Context, Depth, Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::{Axes, Fragment, HAlignment, Regions, Sizing, VAlignment};
use typst_library::model::{EnumElem, ListElem, Numbering, ParElem};
use typst_library::text::TextElem;

use crate::grid::{Cell, CellGrid, GridLayouter};

/// Layout the list.
#[typst_macros::time(span = elem.span())]
pub fn layout_list(
    elem: &Packed<ListElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    let indent = elem.indent(styles);
    let body_indent = elem.body_indent(styles);
    let gutter = elem.spacing(styles).unwrap_or_else(|| {
        if elem.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            ParElem::spacing_in(styles).into()
        }
    });

    let Depth(depth) = ListElem::depth_in(styles);
    let marker = elem
        .marker(styles)
        .resolve(engine, styles, depth)?
        // avoid '#set align' interference with the list
        .aligned(HAlignment::Start + VAlignment::Top);

    let mut cells = vec![];
    let mut locator = locator.split();

    for item in elem.children() {
        cells.push(Cell::new(Content::empty(), locator.next(&())));
        cells.push(Cell::new(marker.clone(), locator.next(&marker.span())));
        cells.push(Cell::new(Content::empty(), locator.next(&())));
        cells.push(Cell::new(
            item.body.clone().styled(ListElem::set_depth(Depth(1))),
            locator.next(&item.body.span()),
        ));
    }

    let grid = CellGrid::new(
        Axes::with_x(&[
            Sizing::Rel(indent.into()),
            Sizing::Auto,
            Sizing::Rel(body_indent.into()),
            Sizing::Auto,
        ]),
        Axes::with_y(&[gutter.into()]),
        cells,
    );
    let layouter = GridLayouter::new(&grid, regions, styles, elem.span());

    layouter.layout(engine)
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
    let numbering = elem.numbering(styles);
    let indent = elem.indent(styles);
    let body_indent = elem.body_indent(styles);
    let gutter = elem.spacing(styles).unwrap_or_else(|| {
        if elem.tight(styles) {
            ParElem::leading_in(styles).into()
        } else {
            ParElem::spacing_in(styles).into()
        }
    });

    let mut cells = vec![];
    let mut locator = locator.split();
    let mut number = elem.start(styles);
    let mut parents = EnumElem::parents_in(styles);

    let full = elem.full(styles);

    // Horizontally align based on the given respective parameter.
    // Vertically align to the top to avoid inheriting `horizon` or `bottom`
    // alignment from the context and having the number be displaced in
    // relation to the item it refers to.
    let number_align = elem.number_align(styles);

    for item in elem.children() {
        number = item.number(styles).unwrap_or(number);

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
        let resolved =
            resolved.aligned(number_align).styled(TextElem::set_overhang(false));

        cells.push(Cell::new(Content::empty(), locator.next(&())));
        cells.push(Cell::new(resolved, locator.next(&())));
        cells.push(Cell::new(Content::empty(), locator.next(&())));
        cells.push(Cell::new(
            item.body.clone().styled(EnumElem::set_parents(smallvec![number])),
            locator.next(&item.body.span()),
        ));
        number = number.saturating_add(1);
    }

    let grid = CellGrid::new(
        Axes::with_x(&[
            Sizing::Rel(indent.into()),
            Sizing::Auto,
            Sizing::Rel(body_indent.into()),
            Sizing::Auto,
        ]),
        Axes::with_y(&[gutter.into()]),
        cells,
    );
    let layouter = GridLayouter::new(&grid, regions, styles, elem.span());

    layouter.layout(engine)
}

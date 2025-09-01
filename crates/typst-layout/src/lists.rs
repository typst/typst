use comemo::Track;
use smallvec::smallvec;
use typst_library::diag::SourceResult;
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Context, Depth, Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::grid::resolve::{Cell, CellGrid};
use typst_library::layout::{Axes, Fragment, HAlignment, Regions, Sizing, VAlignment};
use typst_library::model::{EnumElem, ListElem, Numbering, ParElem, ParbreakElem};
use typst_library::text::TextElem;

use crate::grid::GridLayouter;

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

        cells.push(Cell::new(Content::empty()));
        cells.push(Cell::new(marker.clone()));
        cells.push(Cell::new(Content::empty()));
        cells.push(Cell::new(body.set(ListElem::depth, Depth(1))));
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
    let layouter = GridLayouter::new(&grid, regions, locator, styles, elem.span());

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

        cells.push(Cell::new(Content::empty()));
        cells.push(Cell::new(resolved));
        cells.push(Cell::new(Content::empty()));
        cells.push(Cell::new(body.set(EnumElem::parents, smallvec![number])));
        number =
            if reversed { number.saturating_sub(1) } else { number.saturating_add(1) };
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
    let layouter = GridLayouter::new(&grid, regions, locator, styles, elem.span());

    layouter.layout(engine)
}

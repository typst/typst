use krilla::destination::XyzDestination;
use krilla::outline::{Outline as KrillaOutline, OutlineNode as KrillaOutlineNode};
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::layout::Abs;
use typst_library::model::{HeadingElem, OutlineNode};

use crate::convert::GlobalContext;
use crate::util::AbsExt;

pub(crate) fn build_outline(gc: &GlobalContext) -> KrillaOutline {
    let elems = gc.document.introspector.query(&HeadingElem::ELEM.select());

    let flat = elems
        .iter()
        .map(|elem| {
            let heading = elem.to_packed::<HeadingElem>().unwrap();

            let structural_level = heading.resolve_level(StyleChain::default());

            let level = heading
                .bookmark_level
                .get(StyleChain::default())
                .unwrap_or(structural_level);

            let boomarked = heading
                .bookmarked
                .get(StyleChain::default())
                .unwrap_or_else(|| heading.outlined.get(StyleChain::default()));

            let visible = gc.options.page_ranges.as_ref().is_none_or(|ranges| {
                !ranges.includes_page(
                    gc.document.introspector.page(elem.location().unwrap()),
                )
            });

            let include = boomarked && visible;
            (heading, level, include)
        })
        .collect::<Vec<_>>();

    let tree = OutlineNode::build_tree(flat);

    let mut outline = KrillaOutline::new();
    for child in convert_list(&tree, gc) {
        outline.push_child(child);
    }

    outline
}

fn convert_list(
    nodes: &[OutlineNode<&Packed<HeadingElem>>],
    gc: &GlobalContext,
) -> Vec<KrillaOutlineNode> {
    nodes.iter().flat_map(|node| convert_node(node, gc)).collect()
}

fn convert_node(
    node: &OutlineNode<&Packed<HeadingElem>>,
    gc: &GlobalContext,
) -> Option<KrillaOutlineNode> {
    let loc = node.entry.location().unwrap();
    let pos = gc.document.introspector.position(loc);
    let page_index = pos.page.get() - 1;

    // Prepend the numbers to the title if they exist.
    let text = node.entry.body.plain_text();
    let title = match &node.entry.numbers {
        Some(num) => format!("{num} {text}"),
        None => text.to_string(),
    };

    if let Some(index) = gc.page_index_converter.pdf_page_index(page_index) {
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());
        let dest = XyzDestination::new(
            index,
            krilla::geom::Point::from_xy(pos.point.x.to_f32(), y.to_f32()),
        );

        let mut outline_node = KrillaOutlineNode::new(title, dest);
        for child in convert_list(&node.children, gc) {
            outline_node.push_child(child);
        }

        return Some(outline_node);
    }

    None
}

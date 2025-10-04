use krilla::outline::{Outline as KrillaOutline, OutlineNode as KrillaOutlineNode};
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::model::{HeadingElem, OutlineNode};

use crate::convert::GlobalContext;

pub(crate) fn build_outline(gc: &GlobalContext) -> KrillaOutline {
    let elems = gc.document.introspector.query(&HeadingElem::ELEM.select());

    let flat = elems
        .iter()
        .map(|elem| {
            let heading = elem.to_packed::<HeadingElem>().unwrap();

            let level = heading.resolve_level(StyleChain::default());
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

    // Prepend the numbers to the title if they exist.
    let text = node.entry.body.plain_text();
    let title = match &node.entry.numbers {
        Some(num) => format!("{num} {text}"),
        None => text.to_string(),
    };

    if let Some(dest) = crate::link::pos_to_xyz(&gc.page_index_converter, pos) {
        let mut outline_node = KrillaOutlineNode::new(title, dest);
        for child in convert_list(&node.children, gc) {
            outline_node.push_child(child);
        }

        return Some(outline_node);
    }

    None
}

use std::num::NonZeroUsize;

use krilla::destination::XyzDestination;
use krilla::outline::{Outline, OutlineNode};
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::layout::Abs;
use typst_library::model::HeadingElem;

use crate::convert::GlobalContext;
use crate::util::AbsExt;

pub(crate) fn build_outline(gc: &GlobalContext) -> Outline {
    let mut tree: Vec<HeadingNode> = vec![];

    // Stores the level of the topmost skipped ancestor of the next bookmarked
    // heading. A skipped heading is a heading with 'bookmarked: false', that
    // is, it is not added to the PDF outline, and so is not in the tree.
    // Therefore, its next descendant must be added at its level, which is
    // enforced in the manner shown below.
    let mut last_skipped_level = None;
    let elements = &gc.document.introspector.query(&HeadingElem::ELEM.select());

    for elem in elements.iter() {
        if let Some(page_ranges) = &gc.options.page_ranges
            && !page_ranges
                .includes_page(gc.document.introspector.page(elem.location().unwrap()))
        {
            // Don't bookmark headings in non-exported pages.
            continue;
        }

        let heading = elem.to_packed::<HeadingElem>().unwrap();
        let leaf = HeadingNode::leaf(heading);

        if leaf.bookmarked {
            let mut children = &mut tree;

            // Descend the tree through the latest bookmarked heading of each
            // level until either:
            // - you reach a node whose children would be brothers of this
            // heading (=> add the current heading as a child of this node);
            // - you reach a node with no children (=> this heading probably
            // skipped a few nesting levels in Typst, or one or more ancestors
            // of this heading weren't bookmarked, so add it as a child of this
            // node, which is its deepest bookmarked ancestor);
            // - or, if the latest heading(s) was(/were) skipped
            // ('bookmarked: false'), then stop if you reach a node whose
            // children would be brothers of the latest skipped heading
            // of lowest level (=> those skipped headings would be ancestors
            // of the current heading, so add it as a 'brother' of the least
            // deep skipped ancestor among them, as those ancestors weren't
            // added to the bookmark tree, and the current heading should not
            // be mistakenly added as a descendant of a brother of that
            // ancestor.)
            //
            // That is, if you had a bookmarked heading of level N, a skipped
            // heading of level N, a skipped heading of level N + 1, and then
            // a bookmarked heading of level N + 2, that last one is bookmarked
            // as a level N heading (taking the place of its topmost skipped
            // ancestor), so that it is not mistakenly added as a descendant of
            // the previous level N heading.
            //
            // In other words, a heading can be added to the bookmark tree
            // at most as deep as its topmost skipped direct ancestor (if it
            // exists), or at most as deep as its actual nesting level in Typst
            // (not exceeding whichever is the most restrictive depth limit
            // of those two).
            while children.last().is_some_and(|last| {
                last_skipped_level.is_none_or(|l| last.level < l)
                    && last.level < leaf.level
            }) {
                children = &mut children.last_mut().unwrap().children;
            }

            // Since this heading was bookmarked, the next heading, if it is a
            // child of this one, won't have a skipped direct ancestor (indeed,
            // this heading would be its most direct ancestor, and wasn't
            // skipped). Therefore, it can be added as a child of this one, if
            // needed, following the usual rules listed above.
            last_skipped_level = None;
            children.push(leaf);
        } else if last_skipped_level.is_none_or(|l| leaf.level < l) {
            // Only the topmost / lowest-level skipped heading matters when you
            // have consecutive skipped headings (since none of them are being
            // added to the bookmark tree), hence the condition above.
            // This ensures the next bookmarked heading will be placed
            // at most as deep as its topmost skipped ancestors. Deeper
            // ancestors do not matter as the nesting structure they create
            // won't be visible in the PDF outline.
            last_skipped_level = Some(leaf.level);
        }
    }

    let mut outline = Outline::new();

    for child in convert_nodes(&tree, gc) {
        outline.push_child(child);
    }

    outline
}

#[derive(Debug)]
struct HeadingNode<'a> {
    element: &'a Packed<HeadingElem>,
    level: NonZeroUsize,
    bookmarked: bool,
    children: Vec<HeadingNode<'a>>,
}

impl<'a> HeadingNode<'a> {
    fn leaf(element: &'a Packed<HeadingElem>) -> Self {
        HeadingNode {
            level: element.resolve_level(StyleChain::default()),
            // 'bookmarked' set to 'auto' falls back to the value of 'outlined'.
            bookmarked: element
                .bookmarked
                .get(StyleChain::default())
                .unwrap_or_else(|| element.outlined.get(StyleChain::default())),
            element,
            children: Vec::new(),
        }
    }

    fn to_krilla(&self, gc: &GlobalContext) -> Option<OutlineNode> {
        let loc = self.element.location().unwrap();
        let title = self.element.body.plain_text().to_string();
        let pos = gc.document.introspector.position(loc);
        let page_index = pos.page.get() - 1;

        // Prepend the numbering to title if it exists
        let title = match &self.element.numbering_displayed {
            // The space should be a `h(0.3em)`, but only plain-texts are supported here.
            Some(num) => format!("{num} {title}"),
            None => title,
        };

        if let Some(index) = gc.page_index_converter.pdf_page_index(page_index) {
            let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());
            let dest = XyzDestination::new(
                index,
                krilla::geom::Point::from_xy(pos.point.x.to_f32(), y.to_f32()),
            );

            let mut outline_node = OutlineNode::new(title, dest);
            for child in convert_nodes(&self.children, gc) {
                outline_node.push_child(child);
            }

            return Some(outline_node);
        }

        None
    }
}

fn convert_nodes(nodes: &[HeadingNode], gc: &GlobalContext) -> Vec<OutlineNode> {
    nodes.iter().flat_map(|node| node.to_krilla(gc)).collect()
}

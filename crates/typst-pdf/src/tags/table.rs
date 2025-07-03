use std::num::NonZeroU32;

use krilla::tagging::{TableHeaderScope, Tag, TagKind};
use typst_library::foundations::{Packed, StyleChain};
use typst_library::model::{TableCell, TableElem};

use crate::tags::TagNode;

#[derive(Debug)]
pub struct TableCtx {
    pub table: Packed<TableElem>,
    rows: Vec<Vec<Option<(Packed<TableCell>, TagKind, Vec<TagNode>)>>>,
}

impl TableCtx {
    pub fn new(table: Packed<TableElem>) -> Self {
        Self { table: table.clone(), rows: Vec::new() }
    }

    pub fn insert(&mut self, cell: Packed<TableCell>, nodes: Vec<TagNode>) {
        let x = cell.x.get(StyleChain::default()).unwrap_or_else(|| unreachable!());
        let y = cell.y.get(StyleChain::default()).unwrap_or_else(|| unreachable!());
        let rowspan = cell.rowspan.get(StyleChain::default()).get();
        let colspan = cell.colspan.get(StyleChain::default()).get();

        let tag = {
            // TODO: possibly set internal field on TableCell when resolving
            // the cell grid.
            let is_header = false;
            let rowspan =
                (rowspan != 1).then_some(NonZeroU32::new(rowspan as u32).unwrap());
            let colspan =
                (colspan != 1).then_some(NonZeroU32::new(colspan as u32).unwrap());
            if is_header {
                let scope = TableHeaderScope::Column; // TODO
                Tag::TH(scope).with_row_span(rowspan).with_col_span(colspan).into()
            } else {
                Tag::TD.with_row_span(rowspan).with_col_span(colspan).into()
            }
        };

        let required_height = y + rowspan;
        if self.rows.len() < required_height {
            self.rows.resize_with(required_height, Vec::new);
        }

        let required_width = x + colspan;
        let row = &mut self.rows[y];
        if row.len() < required_width {
            row.resize_with(required_width, || None);
        }

        row[x] = Some((cell, tag, nodes));
    }

    pub fn build_table(self, mut nodes: Vec<TagNode>) -> Vec<TagNode> {
        // Table layouting ensures that there are no overlapping cells, and that
        // any gaps left by the user are filled with empty cells.
        for row in self.rows.into_iter() {
            let mut row_nodes = Vec::new();
            for (_, tag, nodes) in row.into_iter().flatten() {
                row_nodes.push(TagNode::group(tag, nodes));
            }

            // TODO: generate `THead`, `TBody`, and `TFoot`
            nodes.push(TagNode::group(Tag::TR, row_nodes));
        }

        nodes
    }
}

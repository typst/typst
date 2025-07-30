use std::io::Write as _;
use std::num::NonZeroU32;

use az::SaturatingAs;
use krilla::tagging::{Tag, TagId, TagKind};
use smallvec::SmallVec;
use typst_library::foundations::{Packed, Smart};
use typst_library::model::TableCell;
use typst_library::pdf::{TableCellKind, TableHeaderScope};

use crate::tags::util::PropertyValCopied;
use crate::tags::{BBoxCtx, GroupContents, TableId, TagNode};

#[derive(Clone, Debug)]
pub struct TableCtx {
    pub id: TableId,
    pub summary: Option<String>,
    pub bbox: BBoxCtx,
    rows: Vec<Vec<GridCell>>,
    min_width: usize,
}

impl TableCtx {
    pub fn new(id: TableId, summary: Option<String>) -> Self {
        Self {
            id,
            summary,
            bbox: BBoxCtx::new(),
            rows: Vec::new(),
            min_width: 0,
        }
    }

    fn get(&self, x: usize, y: usize) -> Option<&TableCtxCell> {
        let cell = self.rows.get(y)?.get(x)?;
        self.resolve_cell(cell)
    }

    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut TableCtxCell> {
        let cell = self.rows.get_mut(y)?.get_mut(x)?;
        match cell {
            // Reborrow here, so the borrow of `cell` doesn't get returned from
            // the function. Otherwise the borrow checker assumes `cell` borrows
            // `self.rows` for the entirety of the function, not just this match
            // arm, and doesn't allow the second mutable borrow in the match arm
            // below.
            GridCell::Cell(_) => self.rows[y][x].as_cell_mut(),
            &mut GridCell::Spanned(x, y) => self.rows[y][x].as_cell_mut(),
            GridCell::Missing => None,
        }
    }

    pub fn contains(&self, cell: &Packed<TableCell>) -> bool {
        let x = cell.x.val().unwrap_or_else(|| unreachable!());
        let y = cell.y.val().unwrap_or_else(|| unreachable!());
        self.get(x, y).is_some()
    }

    fn resolve_cell<'a>(&'a self, cell: &'a GridCell) -> Option<&'a TableCtxCell> {
        match cell {
            GridCell::Cell(cell) => Some(cell),
            &GridCell::Spanned(x, y) => self.rows[y][x].as_cell(),
            GridCell::Missing => None,
        }
    }

    pub fn insert(&mut self, cell: &Packed<TableCell>, contents: GroupContents) {
        let x = cell.x.val().unwrap_or_else(|| unreachable!());
        let y = cell.y.val().unwrap_or_else(|| unreachable!());
        let rowspan = cell.rowspan.val();
        let colspan = cell.colspan.val();
        let kind = cell.kind.val();

        // Extend the table grid to fit this cell.
        let required_height = y + rowspan.get();
        self.min_width = self.min_width.max(x + colspan.get());
        if self.rows.len() < required_height {
            self.rows
                .resize(required_height, vec![GridCell::Missing; self.min_width]);
        }
        for row in self.rows.iter_mut() {
            if row.len() < self.min_width {
                row.resize_with(self.min_width, || GridCell::Missing);
            }
        }

        // Store references to the cell for all spanned cells.
        for i in y..y + rowspan.get() {
            for j in x..x + colspan.get() {
                self.rows[i][j] = GridCell::Spanned(x, y);
            }
        }

        self.rows[y][x] = GridCell::Cell(TableCtxCell {
            x: x.saturating_as(),
            y: y.saturating_as(),
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            kind,
            headers: SmallVec::new(),
            contents,
        });
    }

    pub fn build_table(mut self, mut contents: GroupContents) -> TagNode {
        // Table layouting ensures that there are no overlapping cells, and that
        // any gaps left by the user are filled with empty cells.
        if self.rows.is_empty() {
            return TagNode::group(Tag::Table.with_summary(self.summary), contents);
        }
        let height = self.rows.len();
        let width = self.rows[0].len();

        // Only generate row groups such as `THead`, `TFoot`, and `TBody` if
        // there are no rows with mixed cell kinds.
        let mut gen_row_groups = true;
        let row_kinds = (self.rows.iter())
            .map(|row| {
                row.iter()
                    .filter_map(|cell| self.resolve_cell(cell))
                    .map(|cell| cell.kind)
                    .fold(Smart::Auto, |a, b| {
                        if let Smart::Custom(TableCellKind::Header(_, scope)) = b {
                            gen_row_groups &= scope == TableHeaderScope::Column;
                        }
                        if let (Smart::Custom(a), Smart::Custom(b)) = (a, b) {
                            gen_row_groups &= a == b;
                        }
                        a.or(b)
                    })
                    .unwrap_or(TableCellKind::Data)
            })
            .collect::<Vec<_>>();

        // Fixup all missing cell kinds.
        for (row, row_kind) in self.rows.iter_mut().zip(row_kinds.iter().copied()) {
            let default_kind =
                if gen_row_groups { row_kind } else { TableCellKind::Data };
            for cell in row.iter_mut() {
                let Some(cell) = cell.as_cell_mut() else { continue };
                cell.kind = cell.kind.or(Smart::Custom(default_kind));
            }
        }

        // Explicitly set the headers attribute for cells.
        for x in 0..width {
            let mut column_header = Vec::new();
            for y in 0..height {
                self.resolve_cell_headers(
                    (x, y),
                    &mut column_header,
                    TableHeaderScope::refers_to_column,
                );
            }
        }
        for y in 0..height {
            let mut row_header = Vec::new();
            for x in 0..width {
                self.resolve_cell_headers(
                    (x, y),
                    &mut row_header,
                    TableHeaderScope::refers_to_row,
                );
            }
        }

        let mut chunk_kind = row_kinds[0];
        let mut row_chunk = Vec::new();
        for (row, row_kind) in self.rows.into_iter().zip(row_kinds) {
            let row_nodes = row
                .into_iter()
                .filter_map(|cell| {
                    let cell = cell.into_cell()?;
                    let rowspan = (cell.rowspan.get() != 1).then_some(cell.rowspan);
                    let colspan = (cell.colspan.get() != 1).then_some(cell.colspan);
                    let tag: TagKind = match cell.unwrap_kind() {
                        TableCellKind::Header(_, scope) => {
                            let id = table_cell_id(self.id, cell.x, cell.y);
                            let scope = table_header_scope(scope);
                            Tag::TH(scope)
                                .with_id(Some(id))
                                .with_headers(Some(cell.headers))
                                .with_row_span(rowspan)
                                .with_col_span(colspan)
                                .into()
                        }
                        TableCellKind::Footer | TableCellKind::Data => Tag::TD
                            .with_headers(Some(cell.headers))
                            .with_row_span(rowspan)
                            .with_col_span(colspan)
                            .into(),
                    };
                    Some(TagNode::group(tag, cell.contents))
                })
                .collect();

            let row = TagNode::virtual_group(Tag::TR, row_nodes);

            // Push the `TR` tags directly.
            if !gen_row_groups {
                contents.nodes.push(row);
                continue;
            }

            // Generate row groups.
            if !should_group_rows(chunk_kind, row_kind) {
                let tag: TagKind = match chunk_kind {
                    TableCellKind::Header(..) => Tag::THead.into(),
                    TableCellKind::Footer => Tag::TFoot.into(),
                    TableCellKind::Data => Tag::TBody.into(),
                };
                let chunk_nodes = std::mem::take(&mut row_chunk);
                contents.nodes.push(TagNode::virtual_group(tag, chunk_nodes));

                chunk_kind = row_kind;
            }
            row_chunk.push(row);
        }

        if !row_chunk.is_empty() {
            let tag: TagKind = match chunk_kind {
                TableCellKind::Header(..) => Tag::THead.into(),
                TableCellKind::Footer => Tag::TFoot.into(),
                TableCellKind::Data => Tag::TBody.into(),
            };
            contents.nodes.push(TagNode::virtual_group(tag, row_chunk));
        }

        let tag = Tag::Table.with_summary(self.summary).with_bbox(self.bbox.get());
        TagNode::group(tag, contents)
    }

    fn resolve_cell_headers<F>(
        &mut self,
        (x, y): (usize, usize),
        current_header: &mut Vec<(NonZeroU32, TagId)>,
        refers_to_dir: F,
    ) where
        F: Fn(&TableHeaderScope) -> bool,
    {
        let table_id = self.id;
        let Some(cell) = self.get_mut(x, y) else { return };

        let mut new_header = None;
        if let TableCellKind::Header(level, scope) = cell.unwrap_kind() {
            if refers_to_dir(&scope) {
                // Remove all headers that are the same or a lower level.
                while current_header.pop_if(|(l, _)| *l >= level).is_some() {}

                let tag_id = table_cell_id(table_id, cell.x, cell.y);
                new_header = Some((level, tag_id));
            }
        }

        if let Some((_, cell_id)) = current_header.last() {
            if !cell.headers.contains(cell_id) {
                cell.headers.push(cell_id.clone());
            }
        }

        current_header.extend(new_header);
    }
}

#[derive(Clone, Debug, Default)]
enum GridCell {
    Cell(TableCtxCell),
    Spanned(usize, usize),
    #[default]
    Missing,
}

impl GridCell {
    fn as_cell(&self) -> Option<&TableCtxCell> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn as_cell_mut(&mut self) -> Option<&mut TableCtxCell> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn into_cell(self) -> Option<TableCtxCell> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }
}

#[derive(Clone, Debug)]
struct TableCtxCell {
    x: u32,
    y: u32,
    rowspan: NonZeroU32,
    colspan: NonZeroU32,
    kind: Smart<TableCellKind>,
    headers: SmallVec<[TagId; 1]>,
    contents: GroupContents,
}

impl TableCtxCell {
    fn unwrap_kind(&self) -> TableCellKind {
        self.kind.unwrap_or_else(|| unreachable!())
    }
}

fn should_group_rows(a: TableCellKind, b: TableCellKind) -> bool {
    match (a, b) {
        (TableCellKind::Header(..), TableCellKind::Header(..)) => true,
        (TableCellKind::Footer, TableCellKind::Footer) => true,
        (TableCellKind::Data, TableCellKind::Data) => true,
        (_, _) => false,
    }
}

fn table_cell_id(table_id: TableId, x: u32, y: u32) -> TagId {
    let mut buf = SmallVec::<[u8; 32]>::new();
    _ = write!(&mut buf, "{}x{x}y{y}", table_id.0);
    TagId::from(buf)
}

fn table_header_scope(scope: TableHeaderScope) -> krilla::tagging::TableHeaderScope {
    match scope {
        TableHeaderScope::Both => krilla::tagging::TableHeaderScope::Both,
        TableHeaderScope::Column => krilla::tagging::TableHeaderScope::Column,
        TableHeaderScope::Row => krilla::tagging::TableHeaderScope::Row,
    }
}

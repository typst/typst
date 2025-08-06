use std::io::Write as _;
use std::num::NonZeroU32;

use az::SaturatingAs;
use krilla::tagging::{Tag, TagId, TagKind};
use smallvec::SmallVec;
use typst_library::foundations::{Packed, Smart};
use typst_library::layout::GridCell;
use typst_library::model::TableCell;
use typst_library::pdf::{TableCellKind, TableHeaderScope};

use crate::tags::util::PropertyValCopied;
use crate::tags::{BBoxCtx, GroupContents, TableId, TagNode};

pub trait GridType {
    type CellData: Clone;
}

#[derive(Clone, Debug)]
pub struct TableData {
    pub id: TableId,
    pub summary: Option<String>,
    pub bbox: BBoxCtx,
}

impl GridType for TableData {
    type CellData = TableCellData;
}

#[derive(Clone, Debug)]
pub struct GridData;

impl GridType for GridData {
    type CellData = GridCellData;
}

#[derive(Clone, Debug)]
pub struct GridCtx<T: GridType> {
    pub data: T,
    rows: Vec<Vec<GridField<T::CellData>>>,
    min_width: usize,
}

impl GridCtx<TableData> {
    pub fn new(id: TableId, summary: Option<String>) -> Self {
        Self {
            data: TableData { id, summary, bbox: BBoxCtx::new() },
            rows: Vec::new(),
            min_width: 0,
        }
    }

    pub fn insert(&mut self, cell: &Packed<TableCell>, contents: GroupContents) {
        let x = cell.x.val().unwrap_or_else(|| unreachable!());
        let y = cell.y.val().unwrap_or_else(|| unreachable!());
        let rowspan = cell.rowspan.val();
        let colspan = cell.colspan.val();
        let kind = cell.kind.val();
        self.insert_cell(CtxCell {
            data: TableCellData { kind, headers: SmallVec::new() },
            x: x.saturating_as(),
            y: y.saturating_as(),
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            contents,
        });
    }

    pub fn build_table(mut self, mut contents: GroupContents) -> TagNode {
        // Table layouting ensures that there are no overlapping cells, and that
        // any gaps left by the user are filled with empty cells.
        if self.rows.is_empty() {
            return TagNode::group(Tag::Table.with_summary(self.data.summary), contents);
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
                    .map(|cell| cell.data.kind)
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
                cell.data.kind = cell.data.kind.or(Smart::Custom(default_kind));
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
                    let tag: TagKind = match cell.data.unwrap_kind() {
                        TableCellKind::Header(_, scope) => {
                            let id = table_cell_id(self.data.id, cell.x, cell.y);
                            let scope = table_header_scope(scope);
                            Tag::TH(scope)
                                .with_id(Some(id))
                                .with_headers(Some(cell.data.headers))
                                .with_row_span(rowspan)
                                .with_col_span(colspan)
                                .into()
                        }
                        TableCellKind::Footer | TableCellKind::Data => Tag::TD
                            .with_headers(Some(cell.data.headers))
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

        let tag = Tag::Table
            .with_summary(self.data.summary)
            .with_bbox(self.data.bbox.get());
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
        let table_id = self.data.id;
        let Some(cell) = self.get_mut(x, y) else { return };

        let mut new_header = None;
        if let TableCellKind::Header(level, scope) = cell.data.unwrap_kind() {
            if refers_to_dir(&scope) {
                // Remove all headers that are the same or a lower level.
                while current_header.pop_if(|(l, _)| *l >= level).is_some() {}

                let tag_id = table_cell_id(table_id, cell.x, cell.y);
                new_header = Some((level, tag_id));
            }
        }

        if let Some((_, cell_id)) = current_header.last() {
            if !cell.data.headers.contains(cell_id) {
                cell.data.headers.push(cell_id.clone());
            }
        }

        current_header.extend(new_header);
    }
}

impl GridCtx<GridData> {
    pub fn new() -> Self {
        Self { data: GridData, rows: Vec::new(), min_width: 0 }
    }

    pub fn insert(&mut self, cell: &Packed<GridCell>, contents: GroupContents) {
        let x = cell.x.val().unwrap_or_else(|| unreachable!());
        let y = cell.y.val().unwrap_or_else(|| unreachable!());
        let rowspan = cell.rowspan.val();
        let colspan = cell.colspan.val();
        self.insert_cell(CtxCell {
            data: GridCellData,
            x: x.saturating_as(),
            y: y.saturating_as(),
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            contents,
        });
    }

    pub fn build_grid(self, mut contents: GroupContents) -> TagNode {
        let cells = (self.rows.into_iter())
            .flat_map(|row| row.into_iter())
            .filter_map(GridField::into_cell)
            .map(|cell| TagNode::group(Tag::Div, cell.contents));

        contents.nodes.extend(cells);

        TagNode::group(Tag::Div, contents)
    }
}

impl<T: GridType> GridCtx<T> {
    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut CtxCell<T::CellData>> {
        let cell = self.rows.get_mut(y)?.get_mut(x)?;
        match cell {
            // Reborrow here, so the borrow of `cell` doesn't get returned from
            // the function. Otherwise the borrow checker assumes `cell` borrows
            // `self.rows` for the entirety of the function, not just this match
            // arm, and doesn't allow the second mutable borrow in the match arm
            // below.
            GridField::Cell(_) => self.rows[y][x].as_cell_mut(),
            &mut GridField::Spanned(x, y) => self.rows[y][x].as_cell_mut(),
            GridField::Missing => None,
        }
    }

    fn resolve_cell<'a>(
        &'a self,
        cell: &'a GridField<T::CellData>,
    ) -> Option<&'a CtxCell<T::CellData>> {
        match cell {
            GridField::Cell(cell) => Some(cell),
            &GridField::Spanned(x, y) => self.rows[y][x].as_cell(),
            GridField::Missing => None,
        }
    }

    fn insert_cell(&mut self, cell: CtxCell<T::CellData>) {
        let x = cell.x as usize;
        let y = cell.y as usize;
        let rowspan = cell.rowspan.get() as usize;
        let colspan = cell.colspan.get() as usize;

        // Extend the table grid to fit this cell.
        let required_height = y + rowspan;
        self.min_width = self.min_width.max(x + colspan);
        if self.rows.len() < required_height {
            self.rows
                .resize(required_height, vec![GridField::Missing; self.min_width]);
        }
        for row in self.rows.iter_mut() {
            if row.len() < self.min_width {
                row.resize_with(self.min_width, || GridField::Missing);
            }
        }

        // Store references to the cell for all spanned cells.
        for i in y..y + rowspan {
            for j in x..x + colspan {
                self.rows[i][j] = GridField::Spanned(x, y);
            }
        }

        self.rows[y][x] = GridField::Cell(cell);
    }
}

#[derive(Clone, Debug, Default)]
enum GridField<D> {
    Cell(CtxCell<D>),
    Spanned(usize, usize),
    #[default]
    Missing,
}

impl<D> GridField<D> {
    fn as_cell(&self) -> Option<&CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn as_cell_mut(&mut self) -> Option<&mut CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn into_cell(self) -> Option<CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }
}

#[derive(Clone, Debug)]
struct CtxCell<D> {
    data: D,
    x: u32,
    y: u32,
    rowspan: NonZeroU32,
    colspan: NonZeroU32,
    contents: GroupContents,
}

#[derive(Clone, Debug)]
pub struct GridCellData;

#[derive(Clone, Debug)]
pub struct TableCellData {
    kind: Smart<TableCellKind>,
    headers: SmallVec<[TagId; 1]>,
}

impl TableCellData {
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
    _ = write!(&mut buf, "{}x{x}y{y}", table_id.get());
    TagId::from(buf)
}

fn table_header_scope(scope: TableHeaderScope) -> krilla::tagging::TableHeaderScope {
    match scope {
        TableHeaderScope::Both => krilla::tagging::TableHeaderScope::Both,
        TableHeaderScope::Column => krilla::tagging::TableHeaderScope::Column,
        TableHeaderScope::Row => krilla::tagging::TableHeaderScope::Row,
    }
}

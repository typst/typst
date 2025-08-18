use std::io::Write as _;
use std::num::NonZeroU32;
use std::ops::Range;
use std::sync::Arc;

use az::SaturatingAs;
use krilla::tagging::{Tag, TagId, TagKind};
use smallvec::SmallVec;
use typst_library::foundations::Packed;
use typst_library::layout::GridCell;
use typst_library::layout::resolve::CellGrid;
use typst_library::model::TableCell;
use typst_library::pdf::{TableCellKind, TableHeaderScope};

use crate::tags::util::PropertyValCopied;
use crate::tags::{BBoxCtx, GroupContents, TableId, TagNode};

#[derive(Clone, Debug)]
pub struct TableCtx {
    pub id: TableId,
    pub summary: Option<String>,
    pub bbox: BBoxCtx,
    pub default_row_kinds: Vec<TableCellKind>,
    grid: Arc<CellGrid>,
    cells: GridCells<TableCellData>,
}

#[derive(Clone, Debug)]
pub struct TableCellData {
    kind: TableCellKind,
    headers: SmallVec<[TagId; 1]>,
}

impl TableCtx {
    pub fn new(grid: Arc<CellGrid>, id: TableId, summary: Option<String>) -> Self {
        let width = grid.non_gutter_column_count();
        let height = grid.non_gutter_row_count();

        let mut grid_headers = grid.headers.iter().peekable();
        let row_kinds = (0..height).map(|y| {
            let grid_y = if grid.has_gutter { 2 * y + 1 } else { y };

            // Find current header
            while grid_headers.next_if(|h| h.range.end <= grid_y).is_some() {}
            if let Some(header) = grid_headers.peek()
                && header.range.contains(&grid_y)
            {
                return TableCellKind::Header(header.level, TableHeaderScope::Column);
            }

            if let Some(footer) = &grid.footer
                && footer.range().contains(&grid_y)
            {
                return TableCellKind::Footer;
            }

            TableCellKind::Data
        });

        Self {
            id,
            summary,
            bbox: BBoxCtx::new(),
            default_row_kinds: row_kinds.collect(),
            grid,
            cells: GridCells::new(width, height),
        }
    }

    pub fn insert(&mut self, cell: &Packed<TableCell>, contents: GroupContents) {
        let x = cell.x.val().unwrap_or_else(|| unreachable!()).saturating_as();
        let y = cell.y.val().unwrap_or_else(|| unreachable!()).saturating_as();
        let rowspan = cell.rowspan.val();
        let colspan = cell.colspan.val();
        let kind = cell.kind.val().unwrap_or_else(|| self.default_row_kinds[y as usize]);
        self.cells.insert(CtxCell {
            data: TableCellData { kind, headers: SmallVec::new() },
            x,
            y,
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            contents,
        });
    }

    pub fn build_table(mut self, mut contents: GroupContents) -> TagNode {
        // Table layouting ensures that there are no overlapping cells, and that
        // any gaps left by the user are filled with empty cells.
        if self.cells.entries.is_empty() {
            return TagNode::group(Tag::Table.with_summary(self.summary), contents);
        }

        let width = self.cells.width();
        let height = self.cells.height();

        // Only generate row groups such as `THead`, `TFoot`, and `TBody` if
        // there are no rows with mixed cell kinds, and there is at least one
        // header or a footer.
        let mut row_kinds = self.default_row_kinds;
        let gen_row_groups = {
            let mut uniform_rows = true;
            let mut has_header_or_footer = false;
            'outer: for (row, row_kind) in self.cells.rows().zip(row_kinds.iter_mut()) {
                *row_kind = self.cells.resolve(row.first().unwrap()).unwrap().data.kind;
                has_header_or_footer |= *row_kind != TableCellKind::Data;
                for cell in row.iter().filter_map(|cell| self.cells.resolve(cell)) {
                    if let TableCellKind::Header(_, scope) = cell.data.kind
                        && scope != TableHeaderScope::Column
                    {
                        uniform_rows = false;
                        break 'outer;
                    }

                    if *row_kind != cell.data.kind {
                        uniform_rows = false;
                        break 'outer;
                    }
                }
            }

            uniform_rows && has_header_or_footer
        };

        // Compute the headers attribute column-wise.
        for x in 0..width {
            let mut column_headers = Vec::new();
            let mut grid_headers = self.grid.headers.iter().peekable();
            for y in 0..height {
                // Find current header region
                let grid_y =
                    if self.grid.has_gutter { 2 * y as usize + 1 } else { y as usize };
                while grid_headers.next_if(|h| h.range.end <= grid_y).is_some() {}
                let region_range = grid_headers.peek().and_then(|header| {
                    if !header.range.contains(&grid_y) {
                        return None;
                    }

                    // Convert from the `CellGrid` coordinates to normal ones.
                    let from_effective =
                        |i: usize| if self.grid.has_gutter { i / 2 } else { i } as u32;
                    let start = from_effective(header.range.start);
                    let end = from_effective(header.range.end);
                    Some(start..end)
                });

                resolve_cell_headers(
                    self.id,
                    &mut self.cells,
                    &mut column_headers,
                    region_range,
                    TableHeaderScope::refers_to_column,
                    (x, y),
                );
            }
        }
        // Compute the headers attribute row-wise.
        for y in 0..height {
            let mut row_headers = Vec::new();
            for x in 0..width {
                resolve_cell_headers(
                    self.id,
                    &mut self.cells,
                    &mut row_headers,
                    None,
                    TableHeaderScope::refers_to_row,
                    (x, y),
                );
            }
        }

        let mut chunk_kind = self.cells.get(0, 0).unwrap().data.kind;
        let mut row_chunk = Vec::new();
        let mut row_iter = self.cells.into_rows();
        while let Some((y, row)) = row_iter.row() {
            let row_nodes = row
                .filter_map(|entry| {
                    let cell = entry.into_cell()?;
                    let rowspan = (cell.rowspan.get() != 1).then_some(cell.rowspan);
                    let colspan = (cell.colspan.get() != 1).then_some(cell.colspan);
                    let tag: TagKind = match cell.data.kind {
                        TableCellKind::Header(_, scope) => {
                            let id = table_cell_id(self.id, cell.x, cell.y);
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
            let row_kind = row_kinds[y as usize];
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
}

struct HeaderCells {
    /// If this header is inside a table header regions defined by a
    /// `table.header()` call, this is the range of that region.
    /// Currently this is only supported for multi row headers.
    region_range: Option<Range<u32>>,
    level: NonZeroU32,
    cell_ids: SmallVec<[TagId; 1]>,
}

fn resolve_cell_headers<F>(
    table_id: TableId,
    cells: &mut GridCells<TableCellData>,
    header_stack: &mut Vec<HeaderCells>,
    region_range: Option<Range<u32>>,
    refers_to_dir: F,
    (x, y): (u32, u32),
) where
    F: Fn(&TableHeaderScope) -> bool,
{
    let Some(cell) = cells.get_mut(x, y) else { return };

    let cell_ids = resolve_cell_header_ids(
        table_id,
        header_stack,
        region_range,
        refers_to_dir,
        cell,
    );

    if let Some(header) = cell_ids {
        for id in header.cell_ids.iter() {
            if !cell.data.headers.contains(id) {
                cell.data.headers.push(id.clone());
            }
        }
    }
}

fn resolve_cell_header_ids<'a, F>(
    table_id: TableId,
    header_stack: &'a mut Vec<HeaderCells>,
    region_range: Option<Range<u32>>,
    refers_to_dir: F,
    cell: &CtxCell<TableCellData>,
) -> Option<&'a HeaderCells>
where
    F: Fn(&TableHeaderScope) -> bool,
{
    let TableCellKind::Header(level, scope) = cell.data.kind else {
        return header_stack.last();
    };
    if !refers_to_dir(&scope) {
        return header_stack.last();
    }

    // Remove all headers with a higher level.
    while header_stack.pop_if(|h| h.level > level).is_some() {}

    let tag_id = table_cell_id(table_id, cell.x, cell.y);

    // Check for multi-row header regions with the same level.
    let Some(prev) = header_stack.last_mut().filter(|h| h.level == level) else {
        header_stack.push(HeaderCells {
            region_range,
            level,
            cell_ids: SmallVec::from_buf([tag_id]),
        });
        return header_stack.iter().rev().nth(1);
    };

    // If the current header region encompasses the cell, add the cell id to
    // the header. This way multiple consecutive header cells in a single header
    // region will be listed for the next cells.
    if prev.region_range.clone().is_some_and(|r| r.contains(&cell.y)) {
        prev.cell_ids.push(tag_id);
    } else {
        // The current region doesn't encompass the cell.
        // Replace the previous heading that had the same level.
        *prev = HeaderCells {
            region_range,
            level,
            cell_ids: SmallVec::from_buf([tag_id]),
        };
    }

    header_stack.iter().rev().nth(1)
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

#[derive(Clone, Debug)]
pub struct GridCtx {
    cells: GridCells<()>,
}

impl GridCtx {
    pub fn new(grid: Arc<CellGrid>) -> Self {
        let width = grid.non_gutter_column_count();
        let height = grid.non_gutter_row_count();
        Self { cells: GridCells::new(width, height) }
    }

    pub fn insert(&mut self, cell: &Packed<GridCell>, contents: GroupContents) {
        let x = cell.x.val().unwrap_or_else(|| unreachable!());
        let y = cell.y.val().unwrap_or_else(|| unreachable!());
        let rowspan = cell.rowspan.val();
        let colspan = cell.colspan.val();
        self.cells.insert(CtxCell {
            data: (),
            x: x.saturating_as(),
            y: y.saturating_as(),
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            contents,
        });
    }

    pub fn build_grid(self, mut contents: GroupContents) -> TagNode {
        let cells = (self.cells.entries.into_iter())
            .filter_map(GridEntry::into_cell)
            .map(|cell| TagNode::group(Tag::Div, cell.contents));

        contents.nodes.extend(cells);

        TagNode::group(Tag::Div, contents)
    }
}

#[derive(Clone, Debug)]
struct GridCells<T> {
    width: usize,
    entries: Vec<GridEntry<T>>,
}

struct RowIter<T> {
    width: u32,
    height: u32,
    consumed: u32,
    inner: std::vec::IntoIter<T>,
}

impl<T> RowIter<T> {
    fn row<'a>(&'a mut self) -> Option<(u32, RowEntryIter<'a, T>)> {
        if self.consumed < self.height {
            let y = self.consumed;
            self.consumed += 1;
            Some((y, RowEntryIter { consumed: 0, parent: self }))
        } else {
            None
        }
    }
}

struct RowEntryIter<'a, T> {
    consumed: u32,
    parent: &'a mut RowIter<T>,
}

// Make sure this iterator consumes the whole row.
impl<T> Drop for RowEntryIter<'_, T> {
    fn drop(&mut self) {
        while self.next().is_some() {}
    }
}

impl<'a, T> Iterator for RowEntryIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.consumed < self.parent.width {
            self.consumed += 1;
            self.parent.inner.next()
        } else {
            None
        }
    }
}

impl<T: Clone> GridCells<T> {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            entries: vec![GridEntry::Missing; width * height],
        }
    }

    fn width(&self) -> u32 {
        self.width as u32
    }

    fn height(&self) -> u32 {
        (self.entries.len() / self.width) as u32
    }

    fn rows(&self) -> impl Iterator<Item = &[GridEntry<T>]> {
        self.entries.chunks(self.width)
    }

    fn into_rows(self) -> RowIter<GridEntry<T>> {
        RowIter {
            width: self.width(),
            height: self.height(),
            consumed: 0,
            inner: self.entries.into_iter(),
        }
    }

    fn get(&self, x: u32, y: u32) -> Option<&CtxCell<T>> {
        let cell = &self.entries[self.cell_idx(x, y)];
        self.resolve(cell)
    }

    fn get_mut(&mut self, x: u32, y: u32) -> Option<&mut CtxCell<T>> {
        let idx = self.cell_idx(x, y);
        let cell = &mut self.entries[idx];
        match cell {
            // Reborrow here, so the borrow of `cell` doesn't get returned from
            // the function. Otherwise the borrow checker assumes `cell` borrows
            // `self.rows` for the entirety of the function, not just this match
            // arm, and doesn't allow the second mutable borrow in the match arm
            // below.
            GridEntry::Cell(_) => self.entries[idx].as_cell_mut(),
            &mut GridEntry::Spanned(idx) => self.entries[idx].as_cell_mut(),
            GridEntry::Missing => None,
        }
    }

    fn resolve<'a>(&'a self, cell: &'a GridEntry<T>) -> Option<&'a CtxCell<T>> {
        match cell {
            GridEntry::Cell(cell) => Some(cell),
            &GridEntry::Spanned(idx) => self.entries[idx].as_cell(),
            GridEntry::Missing => None,
        }
    }

    fn insert(&mut self, cell: CtxCell<T>) {
        let x = cell.x;
        let y = cell.y;
        let rowspan = cell.rowspan.get();
        let colspan = cell.colspan.get();
        let parent_idx = self.cell_idx(x, y);

        // Repeated cells should have their `is_repeated` flag set and be marked
        // as artifacts.
        debug_assert!(self.entries[parent_idx].is_missing());

        // Store references to the cell for all spanned cells.
        for j in y..y + rowspan {
            for i in x..x + colspan {
                let idx = self.cell_idx(i, j);
                self.entries[idx] = GridEntry::Spanned(parent_idx);
            }
        }

        self.entries[parent_idx] = GridEntry::Cell(cell);
    }

    fn cell_idx(&self, x: u32, y: u32) -> usize {
        y as usize * self.width + x as usize
    }
}

#[derive(Clone, Debug, Default)]
enum GridEntry<D> {
    Cell(CtxCell<D>),
    Spanned(usize),
    #[default]
    Missing,
}

impl<D> GridEntry<D> {
    fn as_cell(&self) -> Option<&CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn as_cell_mut(&mut self) -> Option<&mut CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn into_cell(self) -> Option<CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
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

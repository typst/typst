use std::io::Write as _;
use std::num::NonZeroU32;
use std::ops::Range;
use std::sync::Arc;

use az::SaturatingAs;
use krilla::tagging::{self as kt, NaiveRgbColor};
use krilla::tagging::{Tag, TagId, TagKind};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::foundations::Packed;
use typst_library::layout::resolve::{CellGrid, Line, LinePosition};
use typst_library::layout::{Abs, GridCell, Sides};
use typst_library::model::TableCell;
use typst_library::pdf::{TableCellKind, TableHeaderScope};
use typst_library::visualize::{FixedStroke, Stroke};

use crate::tags::convert::TableHeaderScopeExt;
use crate::tags::util::PropertyValCopied;
use crate::tags::{BBoxCtx, GroupContents, Groups, TableId, TagNode, convert};
use crate::util::{AbsExt, SidesExt};

trait GridExt {
    /// Convert from "effective" positions inside the cell grid, which may
    /// include gutter tracks in addition to the cells, to conventional
    /// positions.
    #[allow(clippy::wrong_self_convention)]
    fn from_effective(&self, i: usize) -> u32;

    /// Convert from conventional positions to "effective" positions inside the
    /// cell grid, which may include gutter tracks in addition to the cells.
    fn to_effective(&self, i: u32) -> usize;
}

impl GridExt for CellGrid {
    fn from_effective(&self, i: usize) -> u32 {
        if self.has_gutter { (i / 2) as u32 } else { i as u32 }
    }

    fn to_effective(&self, i: u32) -> usize {
        if self.has_gutter { 2 * i as usize } else { i as usize }
    }
}

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
    stroke: Sides<PrioritzedStroke>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PrioritzedStroke {
    stroke: Option<Arc<Stroke<Abs>>>,
    priority: StrokePriority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StrokePriority {
    GridStroke = 0,
    CellStroke = 1,
    ExplicitLine = 2,
}

impl TableCtx {
    pub fn new(grid: Arc<CellGrid>, id: TableId, summary: Option<String>) -> Self {
        let width = grid.non_gutter_column_count();
        let height = grid.non_gutter_row_count();

        let mut grid_headers = grid.headers.iter().peekable();
        let row_kinds = (0..height as u32).map(|y| {
            let grid_y = grid.to_effective(y);

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

        let [grid_x, grid_y] = [x, y].map(|i| self.grid.to_effective(i));
        let grid_cell = self.grid.cell(grid_x, grid_y).unwrap();
        let stroke = grid_cell.stroke.clone().zip(grid_cell.stroke_overridden).map(
            |(stroke, overriden)| {
                let priority = if overriden {
                    StrokePriority::CellStroke
                } else {
                    StrokePriority::GridStroke
                };
                PrioritzedStroke { stroke, priority }
            },
        );
        self.cells.insert(CtxCell {
            data: TableCellData { kind, headers: SmallVec::new(), stroke },
            x,
            y,
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            contents,
        });
    }

    pub fn build_table(
        mut self,
        groups: &mut Groups,
        contents: GroupContents,
    ) -> TagNode {
        // Table layouting ensures that there are no overlapping cells, and that
        // any gaps left by the user are filled with empty cells.
        // A show rule, can prevent the table from being layed out, in which case
        // all cells will be missing, in that case just return whatever contents
        // that were generated in the show rule.
        if self.cells.entries.iter().all(GridEntry::is_missing) {
            return groups.init_tag(Tag::Table.with_summary(self.summary), contents);
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
                let grid_y = self.grid.to_effective(y);
                while grid_headers.next_if(|h| h.range.end <= grid_y).is_some() {}
                let region_range = grid_headers.peek().and_then(|header| {
                    if !header.range.contains(&grid_y) {
                        return None;
                    }

                    // Convert from the `CellGrid` coordinates to normal ones.
                    let start = self.grid.from_effective(header.range.start);
                    let end = self.grid.from_effective(header.range.end);
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

        // Place h-lines, overwriting the cells stroke.
        place_explicit_lines(
            &mut self.cells,
            &self.grid.hlines,
            height,
            width,
            |cells, (y, x), pos| {
                let cell = cells.cell_mut(x, y)?;
                Some(match pos {
                    LinePosition::Before => &mut cell.data.stroke.bottom,
                    LinePosition::After => &mut cell.data.stroke.top,
                })
            },
        );
        // Place v-lines, overwriting the cells stroke.
        place_explicit_lines(
            &mut self.cells,
            &self.grid.vlines,
            width,
            height,
            |cells, (x, y), pos| {
                let cell = cells.cell_mut(x, y)?;
                Some(match pos {
                    LinePosition::Before => &mut cell.data.stroke.right,
                    LinePosition::After => &mut cell.data.stroke.left,
                })
            },
        );

        // Remove overlapping border strokes between cells.
        for y in 0..self.cells.height() {
            for x in 0..self.cells.width().saturating_sub(1) {
                prioritize_strokes(&mut self.cells, (x, y), (x + 1, y), |a, b| {
                    (&mut a.stroke.right, &mut b.stroke.left)
                });
            }
        }
        for x in 0..self.cells.width() {
            for y in 0..self.cells.height().saturating_sub(1) {
                prioritize_strokes(&mut self.cells, (x, y), (x, y + 1), |a, b| {
                    (&mut a.stroke.bottom, &mut b.stroke.top)
                });
            }
        }

        let (parent_border_thickness, parent_border_color) =
            try_resolve_table_stroke(&self.cells);

        let mut chunk_kind = self.cells.cell(0, 0).unwrap().data.kind;
        let mut row_chunk = Vec::new();
        let mut row_iter = self.cells.into_rows();

        while let Some((y, row)) = row_iter.row() {
            let row_nodes = row
                .filter_map(|entry| {
                    let cell = entry.into_cell()?;
                    let rowspan = (cell.rowspan.get() != 1).then_some(cell.rowspan);
                    let colspan = (cell.colspan.get() != 1).then_some(cell.colspan);
                    let mut tag: TagKind = match cell.data.kind {
                        TableCellKind::Header(_, scope) => {
                            let id = table_cell_id(self.id, cell.x, cell.y);
                            Tag::TH(scope.to_krilla())
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

                    resolve_cell_border_and_background(
                        &self.grid,
                        parent_border_thickness,
                        parent_border_color,
                        [cell.x, cell.y],
                        cell.data.stroke,
                        &mut tag,
                    );

                    Some(groups.init_tag(tag, cell.contents))
                })
                .collect();

            let row = groups.new_virtual(Tag::TR, row_nodes);

            // Push the `TR` tags directly.
            if !gen_row_groups {
                groups.get_mut(contents.id).nodes.push(row);
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
                let node = groups.new_virtual(tag, chunk_nodes);
                groups.get_mut(contents.id).nodes.push(node);

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
            let node = groups.new_virtual(tag, row_chunk);
            groups.get_mut(contents.id).nodes.push(node);
        }

        let tag = Tag::Table
            .with_summary(self.summary)
            .with_bbox(self.bbox.to_krilla())
            .with_border_thickness(parent_border_thickness.map(kt::Sides::uniform))
            .with_border_color(parent_border_color.map(kt::Sides::uniform));
        groups.init_tag(tag, contents)
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
    let Some(cell) = cells.cell_mut(x, y) else { return };

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

fn table_cell_id(table_id: TableId, x: u32, y: u32) -> TagId {
    let mut buf = SmallVec::<[u8; 32]>::new();
    _ = write!(&mut buf, "{}x{x}y{y}", table_id.get());
    TagId::from(buf)
}

fn place_explicit_lines<F>(
    cells: &mut GridCells<TableCellData>,
    lines: &[Vec<Line>],
    block_end: u32,
    inline_end: u32,
    get_side: F,
) where
    F: Fn(
        &mut GridCells<TableCellData>,
        (u32, u32),
        LinePosition,
    ) -> Option<&mut PrioritzedStroke>,
{
    for line in lines.iter().flat_map(|lines| lines.iter()) {
        let end = line.end.map(|n| n.get() as u32).unwrap_or(inline_end);
        let explicit_stroke = || PrioritzedStroke {
            stroke: line.stroke.clone(),
            priority: StrokePriority::ExplicitLine,
        };

        // Fixup line positions before the first, or after the last cell.
        let mut pos = line.position;
        if line.index == 0 {
            pos = LinePosition::After;
        } else if line.index + 1 == block_end as usize {
            pos = LinePosition::Before;
        };

        let block_idx = match pos {
            LinePosition::Before => (line.index - 1) as u32,
            LinePosition::After => line.index as u32,
        };
        for inline_idx in line.start as u32..end {
            if let Some(side) = get_side(cells, (block_idx, inline_idx), pos) {
                *side = explicit_stroke();
            }
        }
    }
}

/// PDF tables don't support gutters, remove all overlapping strokes,
/// that aren't equal. Leave strokes that would overlap but are the same
/// because then only a single value has to be written for `BorderStyle`,
/// `BorderThickness`, and `BorderColor` instead of an array for each.
fn prioritize_strokes<F>(
    cells: &mut GridCells<TableCellData>,
    a: (u32, u32),
    b: (u32, u32),
    get_sides: F,
) where
    F: for<'a> Fn(
        &'a mut TableCellData,
        &'a mut TableCellData,
    ) -> (&'a mut PrioritzedStroke, &'a mut PrioritzedStroke),
{
    let Some([a, b]) = cells.cells_disjoint_mut([a, b]) else { return };

    let (a, b) = get_sides(&mut a.data, &mut b.data);

    // Only remove contesting (different) edge strokes.
    if a.stroke != b.stroke {
        // Prefer the right stroke on same priorities.
        if a.priority <= b.priority {
            a.stroke = b.stroke.clone();
        } else {
            b.stroke = a.stroke.clone();
        }
    }
}

/// Try to resolve a table border stroke color and thickness that is inherited
/// by the cells. In acrobat cells cannot override the border thickness or color
/// of the outer border around the table if the thickness is set.
fn try_resolve_table_stroke(
    cells: &GridCells<TableCellData>,
) -> (Option<f32>, Option<NaiveRgbColor>) {
    // Omitted strokes are counted too for reasons explained above.
    let mut strokes = FxHashMap::<_, usize>::default();
    for cell in cells.entries.iter().filter_map(GridEntry::as_cell) {
        for stroke in cell.data.stroke.iter() {
            *strokes.entry(stroke.stroke.as_ref()).or_default() += 1;
        }
    }

    let uniform_stroke = strokes.len() == 1;

    // Find the most used stroke and convert it to a fixed stroke.
    let stroke = strokes.into_iter().max_by_key(|(_, num)| *num).and_then(|(s, _)| {
        let s = (**s?).clone();
        Some(s.unwrap_or_default())
    });
    let Some(stroke) = stroke else { return (None, None) };

    // Only set a parent stroke width if the table uses one uniform stroke.
    let thickness = uniform_stroke.then_some(stroke.thickness.to_f32());
    let color = convert::paint_to_color(&stroke.paint);

    (thickness, color)
}

fn resolve_cell_border_and_background(
    grid: &CellGrid,
    parent_border_thickness: Option<f32>,
    parent_border_color: Option<NaiveRgbColor>,
    pos: [u32; 2],
    stroke: Sides<PrioritzedStroke>,
    tag: &mut TagKind,
) {
    // Resolve border attributes.
    let fixed = stroke
        .as_ref()
        .map(|s| s.stroke.as_ref().map(|s| (**s).clone().unwrap_or_default()));

    // Acrobat completely ignores the border style attribute, but the spec
    // defines `BorderStyle::None` as the default. So make sure to write
    // the correct border styles.
    let border_style = resolve_sides(&fixed, None, Some(kt::BorderStyle::None), |s| {
        s.map(|s| match s.dash {
            Some(_) => kt::BorderStyle::Dashed,
            None => kt::BorderStyle::Solid,
        })
    });

    // In acrobat `BorderThickness` takes precedence over `BorderStyle`. If
    // A `BorderThickness != 0` is specified for a side the border is drawn
    // even if `BorderStyle::None` is set. So explicitly write zeros for
    // sides that should be omitted.
    let border_thickness =
        resolve_sides(&fixed, parent_border_thickness, Some(0.0), |s| {
            s.map(|s| s.thickness.to_f32())
        });

    let border_color = resolve_sides(&fixed, parent_border_color, None, |s| {
        s.and_then(|s| convert::paint_to_color(&s.paint))
    });

    tag.set_border_style(border_style);
    tag.set_border_thickness(border_thickness);
    tag.set_border_color(border_color);

    let [grid_x, grid_y] = pos.map(|i| grid.to_effective(i));
    let grid_cell = grid.cell(grid_x, grid_y).unwrap();
    let background_color = grid_cell.fill.as_ref().and_then(convert::paint_to_color);
    tag.set_background_color(background_color);
}

/// Try to minimize the attributes written per cell.
/// The parent value will be set on the table tag and is inherited by all table
/// cells. If all present values match the parent or all are missing, the
/// attribute can be omitted, and thus `None` is returned.
/// If one of the present values differs from the parent value, the the cell
/// attribute needs to override the parent attribute, fill up the remaining
/// sides with a `default` value if provided, or any other present value.
///
/// Using an already present value has the benefit of saving storage space in
/// the resulting PDF, if all sides have the same value, because then a
/// [kt::Sides::uniform] value can be written instead of an 4-element array.
fn resolve_sides<F, T>(
    sides: &Sides<Option<FixedStroke>>,
    parent: Option<T>,
    default: Option<T>,
    map: F,
) -> Option<kt::Sides<T>>
where
    T: Copy + PartialEq,
    F: Copy + Fn(Option<&FixedStroke>) -> Option<T>,
{
    let mapped = sides.as_ref().map(|s| map(s.as_ref()));

    if mapped.iter().flatten().all(|v| Some(*v) == parent) {
        // All present values are equal to the parent value.
        return None;
    }

    let Some(first) = mapped.iter().flatten().next() else {
        // All values are missing
        return None;
    };

    // At least one value is different from the parent, fill up the remaining
    // sides with a replacement value.
    let replacement = default.unwrap_or(*first);
    let sides = mapped.unwrap_or(replacement);

    // TODO(accessibility): handle `text(dir: rtl)`
    Some(sides.to_lrtb_krilla())
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

    pub fn build_grid(self, groups: &mut Groups, contents: GroupContents) -> TagNode {
        for cell in self.cells.entries.into_iter().filter_map(GridEntry::into_cell) {
            let node = groups.init_tag(Tag::Div, cell.contents);
            groups.get_mut(contents.id).nodes.push(node);
        }

        groups.init_tag(Tag::Div, contents)
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

    fn cell(&self, x: u32, y: u32) -> Option<&CtxCell<T>> {
        let cell = &self.entries[self.cell_idx(x, y)];
        self.resolve(cell)
    }

    fn cell_mut(&mut self, x: u32, y: u32) -> Option<&mut CtxCell<T>> {
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

    /// Mutably borrows disjoint cells. Cells are considered disjoint if their
    /// positions don't resolve to the same parent cell in case of a
    /// [`GridEntry::Cell`] or indirectly through a [`GridEntry::Spanned`].
    ///
    /// # Panics
    ///
    /// If one of the positions points to a [`GridEntry::Missing`].
    fn cells_disjoint_mut<const N: usize>(
        &mut self,
        positions: [(u32, u32); N],
    ) -> Option<[&mut CtxCell<T>; N]> {
        let indices = positions.map(|(x, y)| {
            let idx = self.cell_idx(x, y);
            let cell = &self.entries[idx];
            match cell {
                GridEntry::Cell(_) => idx,
                &GridEntry::Spanned(idx) => idx,
                GridEntry::Missing => unreachable!(""),
            }
        });

        let entries = self.entries.get_disjoint_mut(indices).ok()?;
        Some(entries.map(|entry| entry.as_cell_mut().unwrap()))
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

        assert!(self.entries[parent_idx].is_missing());

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

use std::io::Write as _;
use std::num::NonZeroU32;
use std::ops::Range;
use std::sync::Arc;

use az::SaturatingAs;
use krilla::tagging as kt;
use krilla::tagging::{NaiveRgbColor, Tag, TagKind};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use typst_library::foundations::Packed;
use typst_library::layout::resolve::{CellGrid, Line, LinePosition};
use typst_library::layout::{Abs, Sides};
use typst_library::model::{TableCell, TableElem};
use typst_library::pdf::{TableCellKind, TableHeaderScope};
use typst_library::visualize::{FixedStroke, Stroke};

use crate::tags::GroupId;
use crate::tags::context::grid::{CtxCell, GridCells, GridEntry, GridExt};
use crate::tags::context::{TableId, TagId};
use crate::tags::tree::Tree;
use crate::tags::util::{self, PropertyOptRef, PropertyValCopied, TableHeaderScopeExt};
use crate::util::{AbsExt, SidesExt};

#[derive(Debug)]
pub struct TableCtx {
    pub table_id: TableId,
    pub elem: Packed<TableElem>,
    row_kinds: Vec<TableCellKind>,
    cells: GridCells<TableCellData>,
    border_thickness: Option<f32>,
    border_color: Option<NaiveRgbColor>,
}

#[derive(Debug, Clone)]
pub struct TableCellData {
    tag: TagId,
    kind: TableCellKind,
    headers: SmallVec<[kt::TagId; 1]>,
    stroke: Sides<PrioritzedStroke>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PrioritzedStroke {
    stroke: Option<Arc<Stroke<Abs>>>,
    priority: StrokePriority,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum StrokePriority {
    GridStroke = 0,
    CellStroke = 1,
    ExplicitLine = 2,
}

impl TableCtx {
    pub fn new(table_id: TableId, table: Packed<TableElem>) -> Self {
        let grid = table.grid.as_ref().unwrap();
        let width = grid.non_gutter_column_count();
        let height = grid.non_gutter_row_count();

        // Generate the default row kinds.
        let mut grid_headers = grid.headers.iter().peekable();
        let default_row_kinds = (0..height as u32)
            .map(|y| {
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
            })
            .collect::<Vec<_>>();

        Self {
            table_id,
            elem: table,
            row_kinds: default_row_kinds,
            cells: GridCells::new(width, height),
            border_thickness: None,
            border_color: None,
        }
    }

    pub fn insert(&mut self, cell: &Packed<TableCell>, tag: TagId, id: GroupId) {
        let x = cell.x.val().unwrap_or_else(|| unreachable!()).saturating_as();
        let y = cell.y.val().unwrap_or_else(|| unreachable!()).saturating_as();
        let rowspan = cell.rowspan.val();
        let colspan = cell.colspan.val();
        let grid = self.elem.grid.as_deref().unwrap();

        let kind = cell.kind.val().unwrap_or(self.row_kinds[y as usize]);

        let [grid_x, grid_y] = [x, y].map(|i| grid.to_effective(i));
        let grid_cell = grid.cell(grid_x, grid_y).unwrap();
        let stroke = grid_cell.stroke.clone().zip(grid_cell.stroke_overridden).map(
            |(stroke, overridden)| {
                let priority = if overridden {
                    StrokePriority::CellStroke
                } else {
                    StrokePriority::GridStroke
                };
                PrioritzedStroke { stroke, priority }
            },
        );
        self.cells.insert(CtxCell {
            data: TableCellData { tag, kind, headers: SmallVec::new(), stroke },
            x,
            y,
            rowspan: rowspan.try_into().unwrap_or(NonZeroU32::MAX),
            colspan: colspan.try_into().unwrap_or(NonZeroU32::MAX),
            id,
        });
    }

    pub fn build_tag(&self) -> TagKind {
        Tag::Table
            .with_summary(self.elem.summary.opt_ref().map(Into::into))
            .with_border_thickness(self.border_thickness.map(kt::Sides::uniform))
            .with_border_color(self.border_color.map(kt::Sides::uniform))
            .into()
    }
}

pub fn build_table(tree: &mut Tree, table_id: TableId, table: GroupId) {
    let table_ctx = tree.ctx.tables.get_mut(table_id);

    // Table layouting ensures that there are no overlapping cells, and that
    // any gaps left by the user are filled with empty cells.
    // A show rule, can prevent the table from being laid out, in which case
    // all cells will be missing, in that case just return whatever contents
    // that were generated in the show rule.
    if table_ctx.cells.iter().all(GridEntry::is_missing) {
        return;
    }

    let width = table_ctx.cells.width();
    let height = table_ctx.cells.height();
    let grid = table_ctx.elem.grid.as_deref().unwrap();

    // Only generate row groups such as `THead`, `TFoot`, and `TBody` if
    // there are no rows with mixed cell kinds, and there is at least one
    // header or a footer.
    let gen_row_groups = {
        let mut uniform_rows = true;
        let mut has_header_or_footer = false;
        let mut has_body = false;
        'outer: for (row, row_kind) in
            table_ctx.cells.rows().zip(table_ctx.row_kinds.iter_mut())
        {
            let first_cell = table_ctx.cells.resolve(row.first().unwrap()).unwrap();
            let first_kind = first_cell.data.kind;

            for cell in row.iter().filter_map(|cell| table_ctx.cells.resolve(cell)) {
                if let TableCellKind::Header(_, scope) = cell.data.kind
                    && scope != TableHeaderScope::Column
                {
                    uniform_rows = false;
                    break 'outer;
                }

                if first_kind != cell.data.kind {
                    uniform_rows = false;
                    break 'outer;
                }
            }

            // If all cells in the row have the same custom kind, the row
            // kind is overwritten.
            *row_kind = first_kind;

            has_header_or_footer |= *row_kind != TableCellKind::Data;
            has_body |= *row_kind == TableCellKind::Data;
        }

        uniform_rows && has_header_or_footer && has_body
    };

    // Compute the headers attribute column-wise.
    for x in 0..width {
        let mut column_headers = Vec::new();
        let mut grid_headers = grid.headers.iter().peekable();
        for y in 0..height {
            // Find current header region
            let grid_y = grid.to_effective(y);
            while grid_headers.next_if(|h| h.range.end <= grid_y).is_some() {}
            let region_range = grid_headers.peek().and_then(|header| {
                if !header.range.contains(&grid_y) {
                    return None;
                }

                // Convert from the `CellGrid` coordinates to normal ones.
                let start = grid.from_effective(header.range.start);
                let end = grid.from_effective(header.range.end);
                Some(start..end)
            });

            resolve_cell_headers(
                table_ctx.table_id,
                &mut table_ctx.cells,
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
                table_ctx.table_id,
                &mut table_ctx.cells,
                &mut row_headers,
                None,
                TableHeaderScope::refers_to_row,
                (x, y),
            );
        }
    }

    // Place h-lines, overwriting the cells stroke.
    place_explicit_lines(
        &mut table_ctx.cells,
        &grid.hlines,
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
        &mut table_ctx.cells,
        &grid.vlines,
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
    for y in 0..height {
        for x in 0..width.saturating_sub(1) {
            prioritize_strokes(&mut table_ctx.cells, (x, y), (x + 1, y), |a, b| {
                (&mut a.stroke.right, &mut b.stroke.left)
            });
        }
    }
    for x in 0..width {
        for y in 0..height.saturating_sub(1) {
            prioritize_strokes(&mut table_ctx.cells, (x, y), (x, y + 1), |a, b| {
                (&mut a.stroke.bottom, &mut b.stroke.top)
            });
        }
    }

    (table_ctx.border_thickness, table_ctx.border_color) =
        try_resolve_table_stroke(&table_ctx.cells);

    let mut chunk_kind = table_ctx.row_kinds[0];
    let mut chunk_id = GroupId::INVALID;
    for (row, y) in table_ctx.cells.rows_mut().zip(0..) {
        let parent = if gen_row_groups {
            let row_kind = table_ctx.row_kinds[y as usize];
            let is_first = chunk_id == GroupId::INVALID;
            if is_first || !should_group_rows(chunk_kind, row_kind) {
                let tag: TagKind = match row_kind {
                    // Only one `THead` group at the start of the table is permitted.
                    TableCellKind::Header(..) if is_first => Tag::THead.into(),
                    TableCellKind::Header(..) => Tag::TBody.into(),
                    TableCellKind::Footer => Tag::TFoot.into(),
                    TableCellKind::Data => Tag::TBody.into(),
                };
                chunk_kind = row_kind;
                chunk_id = tree.groups.push_tag(table, tag);
            }
            chunk_id
        } else {
            table
        };

        let row_id = tree.groups.push_tag(parent, Tag::TR);
        let row_nodes = row
            .iter_mut()
            .filter_map(|entry| {
                let cell = entry.as_cell_mut()?;
                let rowspan = (cell.rowspan.get() != 1).then_some(cell.rowspan);
                let colspan = (cell.colspan.get() != 1).then_some(cell.colspan);
                let cell_kind = cell.data.kind;
                let headers = std::mem::take(&mut cell.data.headers);
                let mut tag: TagKind = match cell_kind {
                    TableCellKind::Header(_, scope) => {
                        let id = table_cell_id(table_ctx.table_id, cell.x, cell.y);
                        Tag::TH(scope.to_krilla())
                            .with_id(Some(id))
                            .with_headers(Some(headers))
                            .with_row_span(rowspan)
                            .with_col_span(colspan)
                            .into()
                    }
                    TableCellKind::Footer | TableCellKind::Data => Tag::TD
                        .with_headers(Some(headers))
                        .with_row_span(rowspan)
                        .with_col_span(colspan)
                        .into(),
                };

                resolve_cell_border_and_background(
                    grid,
                    table_ctx.border_thickness,
                    table_ctx.border_color,
                    [cell.x, cell.y],
                    &cell.data.stroke,
                    &mut tag,
                );

                tree.groups.tags.set(cell.data.tag, tag);

                Some(cell.id)
            })
            .collect::<Vec<_>>();

        tree.groups.extend_groups(row_id, row_nodes.into_iter());
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
    cell_ids: SmallVec<[kt::TagId; 1]>,
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

fn table_cell_id(table_id: TableId, x: u32, y: u32) -> kt::TagId {
    // 32 bytes is the maximum length the ID string can have.
    let mut buf = SmallVec::<[u8; 32]>::new();
    _ = write!(&mut buf, "{}x{x}y{y}", table_id.get() + 1);
    kt::TagId::from(buf)
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
/// by the cells. In Acrobat cells cannot override the border thickness or color
/// of the outer border around the table if the thickness is set.
fn try_resolve_table_stroke(
    cells: &GridCells<TableCellData>,
) -> (Option<f32>, Option<NaiveRgbColor>) {
    // Omitted strokes are counted too for reasons explained above.
    let mut strokes = FxHashMap::<_, usize>::default();
    for cell in cells.iter().filter_map(GridEntry::as_cell) {
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
    let color = util::paint_to_color(&stroke.paint);

    (thickness, color)
}

fn resolve_cell_border_and_background(
    grid: &CellGrid,
    parent_border_thickness: Option<f32>,
    parent_border_color: Option<NaiveRgbColor>,
    pos: [u32; 2],
    stroke: &Sides<PrioritzedStroke>,
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

    // In Acrobat `BorderThickness` takes precedence over `BorderStyle`. If
    // A `BorderThickness != 0` is specified for a side the border is drawn
    // even if `BorderStyle::None` is set. So explicitly write zeros for
    // sides that should be omitted.
    let border_thickness =
        resolve_sides(&fixed, parent_border_thickness, Some(0.0), |s| {
            s.map(|s| s.thickness.to_f32())
        });

    let border_color = resolve_sides(&fixed, parent_border_color, None, |s| {
        s.and_then(|s| util::paint_to_color(&s.paint))
    });

    tag.set_border_style(border_style);
    tag.set_border_thickness(border_thickness);
    tag.set_border_color(border_color);

    let [grid_x, grid_y] = pos.map(|i| grid.to_effective(i));
    let grid_cell = grid.cell(grid_x, grid_y).unwrap();
    let background_color = grid_cell.fill.as_ref().and_then(util::paint_to_color);
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

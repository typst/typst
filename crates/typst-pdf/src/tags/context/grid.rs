use std::num::NonZeroU32;

use az::SaturatingAs;
use typst_library::foundations::Packed;
use typst_library::layout::resolve::CellGrid;
use typst_library::layout::{GridCell, GridElem};

use crate::tags::groups::{GroupId, Groups};
use crate::tags::util::PropertyValCopied;

pub(super) trait GridExt {
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

#[derive(Debug)]
pub struct GridCtx {
    cells: GridCells<()>,
}

impl GridCtx {
    pub fn new(grid: &Packed<GridElem>) -> Self {
        let grid = grid.grid.as_ref().unwrap();
        let width = grid.non_gutter_column_count();
        let height = grid.non_gutter_row_count();
        Self { cells: GridCells::new(width, height) }
    }

    pub fn insert(&mut self, cell: &Packed<GridCell>, id: GroupId) {
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
            id,
        });
    }
}

pub fn build_grid(grid_ctx: &GridCtx, groups: &mut Groups, grid_id: GroupId) {
    for cell in grid_ctx.cells.entries.iter().filter_map(GridEntry::as_cell) {
        groups.push_group(grid_id, cell.id);
    }
}

#[derive(Clone, Debug)]
pub(super) struct GridCells<T> {
    width: usize,
    entries: Vec<GridEntry<T>>,
}

impl<T: Clone> GridCells<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            entries: vec![GridEntry::Missing; width * height],
        }
    }

    pub fn width(&self) -> u32 {
        self.width as u32
    }

    pub fn height(&self) -> u32 {
        (self.entries.len() / self.width) as u32
    }

    pub fn iter(&self) -> impl Iterator<Item = &GridEntry<T>> {
        self.entries.iter()
    }

    pub fn rows(&self) -> impl Iterator<Item = &[GridEntry<T>]> {
        self.entries.chunks(self.width)
    }

    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut [GridEntry<T>]> {
        self.entries.chunks_mut(self.width)
    }

    pub fn cell_mut(&mut self, x: u32, y: u32) -> Option<&mut CtxCell<T>> {
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
    pub fn cells_disjoint_mut<const N: usize>(
        &mut self,
        positions: [(u32, u32); N],
    ) -> Option<[&mut CtxCell<T>; N]> {
        let indices = positions.map(|(x, y)| {
            let idx = self.cell_idx(x, y);
            let cell = &self.entries[idx];
            match cell {
                GridEntry::Cell(_) => idx,
                &GridEntry::Spanned(idx) => idx,
                GridEntry::Missing => unreachable!(),
            }
        });

        let entries = self.entries.get_disjoint_mut(indices).ok()?;
        Some(entries.map(|entry| entry.as_cell_mut().unwrap()))
    }

    pub fn resolve<'a>(&'a self, cell: &'a GridEntry<T>) -> Option<&'a CtxCell<T>> {
        match cell {
            GridEntry::Cell(cell) => Some(cell),
            &GridEntry::Spanned(idx) => self.entries[idx].as_cell(),
            GridEntry::Missing => None,
        }
    }

    pub fn insert(&mut self, cell: CtxCell<T>) {
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
pub(super) enum GridEntry<D> {
    Cell(CtxCell<D>),
    Spanned(usize),
    #[default]
    Missing,
}

impl<D> GridEntry<D> {
    pub fn as_cell(&self) -> Option<&CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    pub fn as_cell_mut(&mut self) -> Option<&mut CtxCell<D>> {
        if let Self::Cell(v) = self { Some(v) } else { None }
    }

    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}

#[derive(Clone, Debug)]
pub(super) struct CtxCell<D> {
    pub data: D,
    pub x: u32,
    pub y: u32,
    pub rowspan: NonZeroU32,
    pub colspan: NonZeroU32,
    pub id: GroupId,
}

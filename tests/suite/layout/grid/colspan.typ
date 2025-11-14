--- grid-colspan paged ---
#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  grid.cell(colspan: 4)[*Full Header*],
  grid.cell(colspan: 2, fill: orange)[*Half*],
  grid.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], grid.cell(colspan: 3, fill: orange.darken(10%))[6],
  grid.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], grid.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  table.cell(colspan: 4)[*Full Header*],
  table.cell(colspan: 2, fill: orange)[*Half*],
  table.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], table.cell(colspan: 3, fill: orange.darken(10%))[6],
  table.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], table.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

--- grid-colspan-gutter paged ---
#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  grid.cell(colspan: 4)[*Full Header*],
  grid.cell(colspan: 2, fill: orange)[*Half*],
  grid.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], grid.cell(colspan: 3, fill: orange.darken(10%))[6],
  grid.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], grid.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  table.cell(colspan: 4)[*Full Header*],
  table.cell(colspan: 2, fill: orange)[*Half*],
  table.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], table.cell(colspan: 3, fill: orange.darken(10%))[6],
  table.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], table.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

--- grid-colspan-thick-stroke paged ---
#set page(width: 300pt)
#table(
  columns: (2em, 2em, auto, auto),
  stroke: 5pt,
  [A], [B], [C], [D],
  table.cell(colspan: 4, lorem(20)),
  [A], table.cell(colspan: 2)[BCBCBCBC], [D]
)

--- grid-colspan-out-of-bounds paged ---
// Error: 3:8-3:32 cell's colspan would cause it to exceed the available column(s)
// Hint: 3:8-3:32 try placing the cell in another position or reducing its colspan
#grid(
  columns: 3,
  [a], grid.cell(colspan: 3)[b]
)

--- grid-colspan-overlap paged ---
// Error: 4:8-4:32 cell would span a previously placed cell at column 2, row 0
// Hint: 4:8-4:32 try specifying your cells in a different order or reducing the cell's rowspan or colspan
#grid(
  columns: 3,
  grid.cell(x: 2, y: 0)[x],
  [a], grid.cell(colspan: 2)[b]
)

--- grid-colspan-over-all-fr-columns paged ---
// Colspan over all fractional columns shouldn't expand auto columns on finite pages
#table(
  columns: (1fr, 1fr, auto),
  [A], [B], [C],
  [D], [E], [F]
)
#table(
  columns: (1fr, 1fr, auto),
  table.cell(colspan: 3, lorem(8)),
  [A], [B], [C],
  [D], [E], [F]
)

--- grid-colspan-over-some-fr-columns paged ---
// Colspan over only some fractional columns will not trigger the heuristic, and
// the auto column will expand more than it should. The table looks off, as a result.
#table(
  columns: (1fr, 1fr, auto),
  [], table.cell(colspan: 2, lorem(8)),
  [A], [B], [C],
  [D], [E], [F]
)

--- grid-colspan-over-all-fr-columns-page-width-auto paged ---
// On infinite pages, colspan over all fractional columns SHOULD expand auto columns
#set page(width: auto)
#table(
  columns: (1fr, 1fr, auto),
  [A], [B], [C],
  [D], [E], [F]
)
#table(
  columns: (1fr, 1fr, auto),
  table.cell(colspan: 3, lorem(8)),
  [A], [B], [C],
  [D], [E], [F]
)

--- grid-colspan-multiple-regions paged ---
// Test multiple regions
#set page(height: 5em)
#grid(
  stroke: red,
  fill: aqua,
  columns: 4,
  [a], [b], [c], [d],
  [a], grid.cell(colspan: 2)[e, f, g, h, i], [f],
  [e], [g], grid.cell(colspan: 2)[eee\ e\ e\ e],
  grid.cell(colspan: 4)[eeee e e e]
)

--- issue-6399-grid-cell-colspan-set-rule paged ---
#set grid.cell(colspan: 2)
#grid(columns: 3, [hehe])

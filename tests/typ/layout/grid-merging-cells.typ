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

---
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

---
#table(
  columns: (2em, 2em, auto, auto),
  stroke: 5pt,
  [A], [B], [C], [D],
  table.cell(colspan: 4, lorem(20)),
  [A], [B], table.cell(colspan: 2)[C, D, E, F]
)

---
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

---
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

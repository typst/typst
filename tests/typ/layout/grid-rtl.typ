// Test RTL grid.

---
#set text(dir: rtl)
- מימין לשמאל

---
#set text(dir: rtl)
#table(columns: 2)[A][B][C][D]

---
// Test interaction between RTL and colspans
#set text(dir: rtl)
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

---
#set text(dir: rtl)
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
// Test multiple regions
#set page(height: 5em)
#set text(dir: rtl)
#grid(
  stroke: red,
  fill: aqua,
  columns: 4,
  [a], [b], [c], [d],
  [a], grid.cell(colspan: 2)[e, f, g, h, i], [f],
  [e], [g], grid.cell(colspan: 2)[eee\ e\ e\ e],
  grid.cell(colspan: 4)[eeee e e e]
)

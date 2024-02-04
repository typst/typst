#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  grid.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], grid.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], grid.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  table.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], table.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], table.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

---
#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  grid.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], grid.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], grid.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  table.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], table.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], table.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

---
// Fixed-size rows
#set page(height: 10em)
#grid(
  columns: 2,
  rows: 1.5em,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  grid.cell(rowspan: 3)[R1], [b],
  [c],
  [d],
  [e], [f],
  grid.cell(rowspan: 5)[R2], [h],
  [i],
  [j],
  [k],
  [l],
  [m], [n]
)

---
// Cell coordinate tests
#set page(height: 10em)
#show table.cell: it => [(#it.x, #it.y)]
#table(
  columns: 3,
  fill: red,
  [a], [b], table.cell(rowspan: 2)[c],
  table.cell(colspan: 2)[d],
  table.cell(colspan: 3, rowspan: 10)[a],
  [b],
)
#table(
  columns: 3,
  gutter: 3pt,
  fill: red,
  [a], [b], table.cell(rowspan: 2)[c],
  table.cell(colspan: 2)[d],
  table.cell(colspan: 3, rowspan: 9)[a],
  [b],
)

---
// Auto row expansion
#set page(height: 10em)
#grid(
  columns: (1em, 1em),
  rows: (0.5em, 0.5em, auto),
  fill: orange,
  gutter: 3pt,
  grid.cell(rowspan: 4, [x x x x] + place(bottom)[*Bot*]),
  [a],
  [b],
  [c],
  [d]
)

---
// Fractional rows
// They cause the auto row to expand more than needed.
#set page(height: 10em)
#grid(
  fill: red,
  gutter: 3pt,
  columns: 3,
  rows: (1em, auto, 1fr),
  [a], [b], grid.cell(rowspan: 3, block(height: 4em, width: 1em, fill: orange)),
  [c], [d],
  [e], [f]
)

---
// Fractional rows
#set page(height: 10em)
#grid(
  fill: red,
  gutter: 3pt,
  columns: 3,
  rows: (1fr, auto, 1em),
  [a], [b], grid.cell(rowspan: 3, block(height: 4em, width: 1em, fill: orange)),
  [c], [d],
  [e], [f]
)

// ---
// #table(
//   columns: 2,
//   rows: (auto, auto, 3em),
//   row-gutter: 1em,
//   fill: red,
//   [a], table.cell(rowspan: 3, block(width: 50%, height: 10em, fill: orange) + place(bottom)[*ZD*]),
//   [e],
//   [f]
// )



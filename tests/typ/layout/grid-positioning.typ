// Test cell positioning in grids.

---
#{
  show grid.cell: it => (it.x, it.y)
  grid(
    columns: 2,
    inset: 5pt,
    fill: red,
    [Hello], [World],
    [Sweet], [Italics]
  )
  grid(
    columns: 2,
    gutter: 3pt,
    [Hello], [World],
    [Sweet], [Italics]
  )
}

---
#grid(
  columns: 2,
  [A], [B],
  grid.cell(x: 1, y: 2)[C], grid.cell(x: 0, y: 2)[D],
  grid.cell(x: 1, y: 1)[E], grid.cell(x: 0, y: 1)[F],
)

---
#grid(
  columns: 3,
  rows: 1.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A],
  grid.cell(x: 2, y: 3)[B]
)

#table(
  columns: (3em, 1em, 3em),
  rows: 1.5em,
  inset: (top: 0pt, bottom: 0pt, rest: 5pt),
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  align: (x, y) => (left, center, right).at(x),
  [A],
  table.cell(x: 2, y: 3)[B]
)

---
// Error: 2-4:2 Attempted to place two different cells at column 0, row 0.
#grid(
  [A],
  grid.cell(x: 0, y: 0)[This shall error]
)

---
#grid(
  columns: 3,
  rows: 1.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A], grid.cell(y: 1)[B], [C], grid.cell(x: auto, y: 1)[D], [E],
  grid.cell(y: 2)[F], grid.cell(x: 0)[G], grid.cell(x: 0, y: auto)[H],
  grid.cell(x: 1)[I]
)

#table(
  columns: 3,
  rows: 1.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A], table.cell(y: 1)[B], [C], table.cell(x: auto, y: 1)[D], [E],
  table.cell(y: 2)[F], table.cell(x: 0)[G], table.cell(x: 0, y: auto)[H],
  table.cell(x: 1)[I]
)

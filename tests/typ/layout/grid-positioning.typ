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
  rows: 2.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A],
  grid.cell(x: 2, y: 3)[B]
)

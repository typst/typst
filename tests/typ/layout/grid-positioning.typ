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

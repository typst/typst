// Test grid styling options.

---
#set page(height: 70pt)
#set grid(fill: (x, y) => if calc.even(x + y) { rgb("aaa") })

#grid(
  columns: (1fr,) * 3,
  stroke: 2pt + rgb("333"),
  [A], [B], [C], [], [], [D \ E \ F \ \ \ G], [H],
)

---
#grid(columns: 3, stroke: none, fill: green, [A], [B], [C])

---
// Test general alignment.
#grid(
  columns: 3,
  align: left,
  [Hello], [Hello], [Hello],
  [A], [B], [C],
)

// Test alignment with a function.
#grid(
  columns: 3,
  align: (x, y) => (left, center, right).at(x),
  [Hello], [Hello], [Hello],
  [A], [B], [C],
)

// Test alignment with array.
#grid(
  columns: (1fr, 1fr, 1fr),
  align: (left, center, right),
  [A], [B], [C]
)

// Test empty array.
#set align(center)
#grid(
  columns: (1fr, 1fr, 1fr),
  align: (),
  [A], [B], [C]
)

a

---
// Test inset.
#grid(
  columns: (1fr,) * 3,
  stroke: 2pt + rgb("333"),
  inset: 5pt,
  [A], [B], [C], [], [], [D \ E \ F \ \ \ G], [H],
)

#grid(
  columns: 3,
  inset: 10pt,
  fill: blue,
  [A], [B], [C]
)

#grid(
  columns: 3,
  inset: (y: 10pt),
  [A], [B], [C]
)

#grid(
  columns: 3,
  inset: (left: 20pt, rest: 10pt),
  stroke: 3pt + red,
  [A], [B], [C]
)

#grid(
  columns: 2,
  inset: (
    left: 20pt,
    right: 5pt,
    top: 10pt,
    bottom: 3pt,
  ),
  [A],
  [B],
)

---
// Cell override
#grid(
  align: left,
  fill: red,
  stroke: blue,
  inset: 5pt,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  grid.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A],
  grid.cell(align: horizon)[G2], [A\ A\ A],
  grid.cell(inset: 0pt)[I], [F],
  [H], grid.cell(fill: blue)[J]
)

---
// Cell show rule
#show grid.cell: it => [Zz]

#grid(
  align: left,
  fill: red,
  stroke: blue,
  inset: 5pt,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  grid.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A]
)

---
#show grid.cell: it => (it.align, it.fill)
#grid(
  align: left,
  row-gutter: 5pt,
  [A],
  grid.cell(align: right)[B],
  grid.cell(fill: blue)[B],
)

---
// Cell set rules
#set grid.cell(align: center)
#show grid.cell: it => (it.align, it.fill, it.inset)
#set grid.cell(inset: 20pt)
#grid(
  align: left,
  row-gutter: 5pt,
  [A],
  grid.cell(align: right)[B],
  grid.cell(fill: blue)[B],
)

---
// First doc example
#grid(
  columns: 2,
  fill: red,
  align: left,
  inset: 5pt,
  [ABC], [ABC],
  grid.cell(fill: blue)[C], [D],
  grid.cell(align: center)[E], [F],
  [G], grid.cell(inset: 0pt)[H]
)

---
// Second doc example
#{
  show grid.cell: emph
  grid(
    columns: 2,
    gutter: 3pt,
    [Hello], [World],
    [Sweet], [Italics]
  )
}

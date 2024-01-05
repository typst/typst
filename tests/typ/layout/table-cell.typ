// Test basic styling using the table.cell element.

---
// Cell override
#table(
  align: left,
  fill: red,
  stroke: blue,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  table.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A],
  table.cell(align: horizon)[G2], [A\ A\ A],
  table.cell(inset: 0pt)[I], [F],
  [H], table.cell(fill: blue)[J]
)

---
// Cell show rule
#show table.cell: it => [Zz]

#table(
  align: left,
  fill: red,
  stroke: blue,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  table.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A]
)

---
#show table.cell: it => (it.align, it.fill)
#table(
  align: left,
  row-gutter: 5pt,
  [A],
  table.cell(align: right)[B],
  table.cell(fill: aqua)[B],
)

---
// Cell set rules
#set table.cell(align: center)
#show table.cell: it => (it.align, it.fill, it.inset)
#set table.cell(inset: 20pt)
#table(
  align: left,
  row-gutter: 5pt,
  [A],
  table.cell(align: right)[B],
  table.cell(fill: aqua)[B],
)

---
// Test folding per-cell properties (align and inset)
#table(
  columns: (1fr, 1fr),
  rows: (2.5em, auto),
  align: right,
  fill: (x, y) => (green, aqua).at(calc.rem(x + y, 2)),
  [Top], table.cell(align: bottom)[Bot],
  table.cell(inset: (bottom: 0pt))[Bot], table.cell(inset: (bottom: 0pt))[Bot]
)

---
// Test overriding outside alignment
#set align(bottom + right)
#table(
  columns: (1fr, 1fr),
  rows: 2em,
  align: auto,
  fill: green,
  [BR], [BR],
  table.cell(align: left, fill: aqua)[BL], table.cell(align: top, fill: red.lighten(50%))[TR]
)

---
// First doc example
#table(
  columns: 2,
  fill: green,
  align: right,
  [*Name*], [*Data*],
  table.cell(fill: blue)[J.], [Organizer],
  table.cell(align: center)[K.], [Leader],
  [M.], table.cell(inset: 0pt)[Player]
)

---
#{
  show table.cell: emph
  table(
    columns: 2,
    [Person], [Animal],
    [John], [Dog]
  )
}

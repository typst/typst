// Test tables.

---
#set page(height: 70pt)
#set table(fill: (x, y) => if calc.even(x + y) { rgb("aaa") })

#table(
  columns: (1fr,) * 3,
  stroke: 2pt + rgb("333"),
  [A], [B], [C], [], [], [D \ E \ F \ \ \ G], [H],
)

---
#table(columns: 3, stroke: none, fill: green, [A], [B], [C])

---
// Test alignment with array.
#table(
  columns: (1fr, 1fr, 1fr),
  align: (left, center, right),
  [A], [B], [C]
)

// Test empty array.
#set align(center)
#table(
  columns: (1fr, 1fr, 1fr),
  align: (),
  [A], [B], [C]
)

---
// Test inset.
#table(
  columns: 3,
  inset: 10pt,
  [A], [B], [C]
)

#table(
  columns: 3,
  inset: (y: 10pt),
  [A], [B], [C]
)

#table(
  columns: 3,
  inset: (left: 20pt, rest: 10pt),
  [A], [B], [C]
)

#table(
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
// Test interaction with gutters.
#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  row-gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  column-gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

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
  table.cell(fill: blue)[B],
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
  table.cell(fill: blue)[B],
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
// Second doc example
#{
  show table.cell: emph
  table(
    columns: 2,
    [Person], [Animal],
    [John], [Dog]
  )
}

---
// Ref: false
#table()

---
// Error: 14-19 expected color, gradient, pattern, none, array, or function, found string
#table(fill: "hey")

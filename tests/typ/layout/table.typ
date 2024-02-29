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

#table(
  columns: 3,
  fill: (x, y) => (if y == 0 { aqua } else { orange }).darken(x * 15%),
  inset: (x, y) => (left: if x == 0 { 0pt } else { 5pt }, right: if x == 0 { 5pt } else { 0pt }, y: if y == 0 { 0pt } else { 5pt }),
  [A], [B], [C],
  [A], [B], [C],
)

#table(
  columns: 3,
  inset: (0pt, 5pt, 10pt),
  fill: (x, _) => aqua.darken(x * 15%),
  [A], [B], [C],
)

---
// Test inset folding
#set table(inset: 10pt)
#set table(inset: (left: 0pt))

#table(
  fill: red,
  inset: (right: 0pt),
  table.cell(inset: (top: 0pt))[a]
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
// Ref: false
#table()

---
// Error: 14-19 expected color, gradient, pattern, none, array, or function, found string
#table(fill: "hey")

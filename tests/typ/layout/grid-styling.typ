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
// Test interaction with gutters.
#grid(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#grid(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  row-gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#grid(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  column-gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#grid(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

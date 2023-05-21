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
// Ref: false
#table()

---
// Error: 14-19 expected color, none, array, or function, found string
#table(fill: "hey")

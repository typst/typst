// Test tables.

---
#set page(height: 70pt)
#set table(fill: (x, y) => if even(x + y) { rgb("aaa") })

#table(
  columns: (1fr,) * 3,
  stroke: 2pt + rgb("333"),
  [A], [B], [C], [], [], [D \ E \ F \ \ \ G], [H],
)

---
#table(columns: 3, stroke: none, fill: green, [A], [B], [C])

---
// Ref: false
#table()

---
// Error: 14-19 expected color or none or function, found string
#table(fill: "hey")

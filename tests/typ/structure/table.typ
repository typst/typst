// Test tables.

---
#set page(height: 70pt)
#set table(primary: rgb("aaa"), secondary: none)

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

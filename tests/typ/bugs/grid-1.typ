// Test that grid base for auto rows makes sense.

---
#set page(height: 150pt)
#table(
  columns: (1.5cm, auto),
  rows: (auto, auto),
  rect(width: 100%, fill: red),
  rect(width: 100%, fill: blue),
  rect(width: 100%, height: 50%, fill: green),
)

---
#rect(width: 100%, height: 1em)
- #rect(width: 100%, height: 1em)
  - #rect(width: 100%, height: 1em)

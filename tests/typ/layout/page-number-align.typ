// Test page number alignment.

---
#set page(
  height: 100pt,
  margin: 30pt,
  numbering: "(1)",
  number-align: top + right,
)

#block(width: 100%, height: 100%, fill: aqua.lighten(50%))

---
#set page(
  height: 100pt,
  margin: 30pt,
  numbering: "[1]",
  number-align: bottom + left,
)

#block(width: 100%, height: 100%, fill: aqua.lighten(50%))

---
// Error: 25-39 page number cannot be `horizon`-aligned
#set page(number-align: left + horizon)

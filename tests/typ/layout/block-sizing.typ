// Test blocks with fixed height.

---
#set page(height: 100pt)
#set align(center)

#lorem(10)
#block(width: 80%, height: 60pt, fill: aqua)
#lorem(6)
#block(
  breakable: false,
  width: 100%,
  inset: 4pt,
  fill: aqua,
  lorem(8) + colbreak(),
)

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

---
// Layout inside a block with certain dimensions should provide those dimensions.

#set page(height: 120pt)
#block(width: 60pt, height: 80pt, layout(size => [
  This block has a width of #size.width and height of #size.height
]))

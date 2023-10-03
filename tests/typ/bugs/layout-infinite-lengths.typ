// Test that passing infinite lengths to drawing primitives does not crash Typst.

---
#set page(width: auto, height: auto)

// Error: 1-1 cannot create grid with infinite size
#layout(size => grid(columns: (size.width, size.height))[a][b][c][d])

---
#set page(width: auto, height: auto)

// Error: 1-1 cannot create grid with infinite size
#layout(size => grid(rows: (size.width, size.height))[a][b][c][d])

---
#set page(width: auto, height: auto)

// Error: 1-1 cannot create line with infinite length
#layout(size => line(length: size.width))

---
#set page(width: auto, height: auto)

// Error: 1-1 cannot create polygon with infinite size
#layout(size => polygon((0pt,0pt), (0pt, size.width)))

// Test that passing infinite lengths to drawing primitives does not crash Typst.

---
#set page(width: auto, height: auto)

// Error: cannot expand into infinite width
#layout(size => grid(columns: (size.width, size.height))[a][b][c][d])

---
#set page(width: auto, height: auto)

// Error: 17-66 cannot create grid with infinite height
#layout(size => grid(rows: (size.width, size.height))[a][b][c][d])

---
#set page(width: auto, height: auto)

// Error: 17-41 cannot create line with infinite length
#layout(size => line(length: size.width))

---
#set page(width: auto, height: auto)

// Error: 17-54 cannot create polygon with infinite size
#layout(size => polygon((0pt,0pt), (0pt, size.width)))

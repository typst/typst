// Test how the layout engine reacts when reaching limits like
// zero, infinity or when dealing with NaN.

--- issue-1216-clamp-panic render ---
#set page(height: 20pt, margin: 0pt)
#v(22pt)
#block(fill: red, width: 100%, height: 10pt, radius: 4pt)

--- issue-1918-layout-infinite-length-grid-columns render ---
// Test that passing infinite lengths to drawing primitives does not crash Typst.
#set page(width: auto, height: auto)

// Error: 58-59 cannot expand into infinite width
#layout(size => grid(columns: (size.width, size.height))[a][b][c][d])

--- issue-1918-layout-infinite-length-grid-rows render ---
#set page(width: auto, height: auto)

// Error: 17-66 cannot create grid with infinite height
#layout(size => grid(rows: (size.width, size.height))[a][b][c][d])

--- issue-1918-layout-infinite-length-line render ---
#set page(width: auto, height: auto)

// Error: 17-41 cannot create line with infinite length
#layout(size => line(length: size.width))

--- issue-1918-layout-infinite-length-polygon render ---
#set page(width: auto, height: auto)

// Error: 17-54 cannot create polygon with infinite size
#layout(size => polygon((0pt,0pt), (0pt, size.width)))

// Test RTL grid.

--- list-rtl ---
#set text(dir: rtl)
- מימין לשמאל

--- grid-rtl ---
#set text(dir: rtl)
#table(columns: 2)[A][B][C][D]

--- grid-rtl-colspan ---
// Test interaction between RTL and colspans
#set text(dir: rtl)
#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  grid.cell(colspan: 4)[*Full Header*],
  grid.cell(colspan: 2, fill: orange)[*Half*],
  grid.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], grid.cell(colspan: 3, fill: orange.darken(10%))[6],
  grid.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], grid.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  grid.cell(colspan: 4)[*Full Header*],
  grid.cell(colspan: 2, fill: orange)[*Half*],
  grid.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], grid.cell(colspan: 3, fill: orange.darken(10%))[6],
  grid.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], grid.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

--- grid-rtl-colspan-stroke ---
#set text(dir: rtl)
#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  table.cell(colspan: 4)[*Full Header*],
  table.cell(colspan: 2, fill: orange)[*Half*],
  table.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], table.cell(colspan: 3, fill: orange.darken(10%))[6],
  table.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], table.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  table.cell(colspan: 4)[*Full Header*],
  table.cell(colspan: 2, fill: orange)[*Half*],
  table.cell(colspan: 2, fill: orange.darken(10%))[*Half*],
  [*A*], [*B*], [*C*], [*D*],
  [1], [2], [3], [4],
  [5], table.cell(colspan: 3, fill: orange.darken(10%))[6],
  table.cell(colspan: 2, fill: orange)[7], [8], [9],
  [10], table.cell(colspan: 2, fill: orange.darken(10%))[11], [12]
)

--- grid-rtl-multiple-regions ---
// Test multiple regions
#set page(height: 5em)
#set text(dir: rtl)
#grid(
  stroke: red,
  fill: aqua,
  columns: 4,
  [a], [b], [c], [d],
  [a], grid.cell(colspan: 2)[e, f, g, h, i], [f],
  [e], [g], grid.cell(colspan: 2)[eee\ e\ e\ e],
  grid.cell(colspan: 4)[eeee e e e]
)

--- grid-rtl-vline-position ---
// Test left and right for vlines in RTL
#set text(dir: rtl)
#grid(
  columns: 3,
  inset: 5pt,
  grid.vline(stroke: red, position: left), grid.vline(stroke: green, position: right), [a],
  grid.vline(stroke: red, position: left), grid.vline(stroke: 2pt, position: right), [b],
  grid.vline(stroke: red, position: left), grid.vline(stroke: 2pt, position: right), [c],
  grid.vline(stroke: aqua, position: right)
)

#grid(
  columns: 3,
  inset: 5pt,
  gutter: 3pt,
  grid.vline(stroke: green, position: left), grid.vline(stroke: red, position: right), [a],
  grid.vline(stroke: blue, position: left), grid.vline(stroke: red, position: right), [b],
  grid.vline(stroke: blue, position: left), grid.vline(stroke: red, position: right), [c],
  grid.vline(stroke: 2pt, position: right)
)

#grid(
  columns: 3,
  inset: 5pt,
  grid.vline(stroke: green, position: start), grid.vline(stroke: red, position: end), [a],
  grid.vline(stroke: 2pt, position: start), grid.vline(stroke: red, position: end), [b],
  grid.vline(stroke: 2pt, position: start), grid.vline(stroke: red, position: end), [c],
  grid.vline(stroke: 2pt, position: start)
)

#grid(
  columns: 3,
  inset: 5pt,
  gutter: 3pt,
  grid.vline(stroke: green, position: start), grid.vline(stroke: red, position: end), [a],
  grid.vline(stroke: blue, position: start), grid.vline(stroke: red, position: end), [b],
  grid.vline(stroke: blue, position: start), grid.vline(stroke: red, position: end), [c],
  grid.vline(stroke: 2pt, position: start)
)

--- grid-rtl-vline-out-of-bounds ---
// Error: 3:8-3:34 cannot place vertical line at the 'end' position of the end border (x = 1)
// Hint: 3:8-3:34 set the line's position to 'start' or place it at a smaller 'x' index
#set text(dir: rtl)
#grid(
  [a], grid.vline(position: left)
)

--- grid-rtl-complex ---
#set text(dir: rtl)

#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  grid.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], grid.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], grid.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

#table(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
  table.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], table.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], table.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

--- grid-rtl-rowspan ---
#set page(height: 10em)
#set text(dir: rtl)
#table(
  columns: 2,
  rows: (auto, auto, 3em),
  row-gutter: 1em,
  fill: red,
  [a], table.cell(rowspan: 3, block(width: 50%, height: 10em, fill: orange) + place(bottom)[*ZD*]),
  [e],
  [f]
)

--- grid-rtl-header ---
// Headers
#set page(height: 15em)
#set text(dir: rtl)
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
    table.cell(stroke: red)[*N1*], table.cell(stroke: aqua)[*N2*], [*D1*], [*D2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..range(0, 10).map(i => ([\##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
)

--- grid-rtl-counter ---
// Test interaction between RTL and counters
#set text(dir: rtl)
#let test = counter("test")
#grid(
  columns: (1fr, 1fr),
  inset: 5pt,
  align: center,
  [
    a: // should produce 1
    #test.step()
    #context test.get().first()
  ],
  [
    b: // should produce 2
    #test.step()
    #context test.get().first()
  ],
)

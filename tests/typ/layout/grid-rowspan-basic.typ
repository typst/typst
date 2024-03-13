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
  table.cell(rowspan: 2, fill: orange)[*Left*],
  [Right A], [Right A], [Right A],
  [Right B], table.cell(colspan: 2, rowspan: 2, fill: orange.darken(10%))[B Wide],
  [Left A], [Left A],
  [Left B], [Left B], table.cell(colspan: 2, rowspan: 3, fill: orange)[Wide and Long]
)

---
#grid(
  columns: 4,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  inset: 5pt,
  align: center,
  gutter: 3pt,
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

---
// Fixed-size rows
#set page(height: 10em)
#grid(
  columns: 2,
  rows: 1.5em,
  fill: (x, y) => if calc.odd(x + y) { blue.lighten(50%) } else { blue.lighten(10%) },
  grid.cell(rowspan: 3)[R1], [b],
  [c],
  [d],
  [e], [f],
  grid.cell(rowspan: 5)[R2], [h],
  [i],
  [j],
  [k],
  [l],
  [m], [n]
)

---
// Cell coordinate tests
#set page(height: 10em)
#show table.cell: it => [(#it.x, #it.y)]
#table(
  columns: 3,
  fill: red,
  [a], [b], table.cell(rowspan: 2)[c],
  table.cell(colspan: 2)[d],
  table.cell(colspan: 3, rowspan: 10)[a],
  table.cell(colspan: 2)[b],
)
#table(
  columns: 3,
  gutter: 3pt,
  fill: red,
  [a], [b], table.cell(rowspan: 2)[c],
  table.cell(colspan: 2)[d],
  table.cell(colspan: 3, rowspan: 9)[a],
  table.cell(colspan: 2)[b],
)

---
// Auto row expansion
#set page(height: 10em)
#grid(
  columns: (1em, 1em),
  rows: (0.5em, 0.5em, auto),
  fill: orange,
  gutter: 3pt,
  grid.cell(rowspan: 4, [x x x x] + place(bottom)[*Bot*]),
  [a],
  [b],
  [c],
  [d]
)

---
// Excessive rowspan (no gutter)
#set page(height: 10em)
#table(
  columns: 4,
  fill: red,
  [a], [b], table.cell(rowspan: 2)[c], [d],
  table.cell(colspan: 2, stroke: (bottom: aqua + 2pt))[e], table.cell(stroke: (bottom: aqua))[f],
  table.cell(colspan: 2, rowspan: 10)[R1], table.cell(colspan: 2, rowspan: 10)[R2],
  [b],
)

---
// Excessive rowspan (with gutter)
#set page(height: 10em)
#table(
  columns: 4,
  gutter: 3pt,
  fill: red,
  [a], [b], table.cell(rowspan: 2)[c], [d],
  table.cell(colspan: 2, stroke: (bottom: aqua + 2pt))[e], table.cell(stroke: (bottom: aqua))[f],
  table.cell(colspan: 2, rowspan: 10)[R1], table.cell(colspan: 2, rowspan: 10)[R2],
  [b],
)

---
// Fractional rows
// They cause the auto row to expand more than needed.
#set page(height: 10em)
#grid(
  fill: red,
  gutter: 3pt,
  columns: 3,
  rows: (1em, auto, 1fr),
  [a], [b], grid.cell(rowspan: 3, block(height: 4em, width: 1em, fill: orange)),
  [c], [d],
  [e], [f]
)

---
// Fractional rows
#set page(height: 10em)
#grid(
  fill: red,
  gutter: 3pt,
  columns: 3,
  rows: (1fr, auto, 1em),
  [a], [b], grid.cell(rowspan: 3, block(height: 4em, width: 1em, fill: orange)),
  [c], [d],
  [e], [f]
)

---
// Cell order
#let count = counter("count")
#show grid.cell: it => {
  count.step()
  count.display()
}

#grid(
  columns: (2em,) * 3,
  stroke: aqua,
  rows: 1.2em,
  fill: (x, y) => if calc.odd(x + y) { red } else { orange },
  [a], grid.cell(rowspan: 2)[b], grid.cell(rowspan: 2)[c],
  [d],
  grid.cell(rowspan: 2)[f], [g], [h],
  [i], [j],
  [k], [l], [m],
  grid.cell(rowspan: 2)[n], [o], [p],
  [q], [r],
  [s], [t], [u]
)

---
#table(
    columns: 3,
    rows: (auto, auto, auto, 2em),
    gutter: 3pt,
    table.cell(rowspan: 4)[a \ b\ c\ d\ e], [c], [d],
    [e], table.cell(breakable: false, rowspan: 2)[f],
    [g]
)

---
// Test cell breakability
#show grid.cell: it => {
  assert.eq(it.breakable, (it.x, it.y) != (0, 6) and (it.y in (2, 5, 6) or (it.x, it.y) in ((0, 1), (2, 3), (1, 7))))
  it.breakable
}
#grid(
  columns: 3,
  rows: (6pt, 1fr, auto, 1%, 1em, auto, auto, 0.2in),
  row-gutter: (0pt, 0pt, 0pt, auto),
  [a], [b], [c],
  grid.cell(rowspan: 3)[d], [e], [f],
  [g], [h],
  [i], grid.cell(rowspan: 2)[j],
  [k],
  grid.cell(y: 5)[l],
  grid.cell(y: 6, breakable: false)[m], grid.cell(y: 6, breakable: true)[n],
  grid.cell(y: 7, breakable: false)[o], grid.cell(y: 7, breakable: true)[p], grid.cell(y: 7, breakable: auto)[q]
)

---
#table(
  columns: 2,
  table.cell(stroke: (bottom: red))[a], [b],
  table.hline(stroke: green),
  table.cell(stroke: (top: yellow, left: green, right: aqua, bottom: blue), colspan: 1, rowspan: 2)[d], table.cell(colspan: 1, rowspan: 2)[e],
  [f],
  [g]
)

---
#table(
  columns: 2,
  gutter: 3pt,
  table.cell(stroke: (bottom: red))[a], [b],
  table.hline(stroke: green),
  table.cell(stroke: (top: yellow, left: green, right: aqua, bottom: blue), colspan: 1, rowspan: 2)[d], table.cell(colspan: 1, rowspan: 2)[e],
  [f],
  [g]
)

---
// Block below shouldn't expand to the end of the page, but stay within its
// rows' boundaries.
#set page(height: 9em)
#table(
  rows: (1em, 1em, 1fr, 1fr, auto),
  table.cell(rowspan: 2, block(width: 2em, height: 100%, fill: red)),
  table.cell(rowspan: 2, block(width: 2em, height: 100%, fill: red)),
  [a]
)

---
#set page(height: 7em)
#table(
  columns: 3,
  [], [], table.cell(breakable: true, rowspan: 2, block(width: 2em, height: 100%, fill: red)),
  table.cell(breakable: false, block(width: 2em, height: 100%, fill: red)),
  table.cell(breakable: false, rowspan: 2, block(width: 2em, height: 100%, fill: red)),
)

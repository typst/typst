#let double-line = pattern(size: (1.5pt, 1.5pt), {
  place(line(stroke: .6pt, start: (0%, 50%), end: (100%, 50%)))
})

#table(
  stroke: (_, y) => if y != 1 { (bottom: black) },
  columns: 3,
  table.cell(colspan: 3, align: center)[*Epic Table*],
  align(center)[*Name*], align(center)[*Age*], align(center)[*Data*],
  table.hline(stroke: (paint: double-line, thickness: 2pt)),
  [John], [30], [None],
  [Martha], [20], [A],
  [Joseph], [35], [D]
)

---
// Test folding
#set grid(stroke: red)
#set grid(stroke: 5pt)

#grid(
  inset: 10pt,
  columns: 2,
  stroke: stroke(dash: "loosely-dotted"),
  grid.vline(start: 2, end: 3, stroke: (paint: green, dash: none)),
  [a], [b],
  grid.hline(end: 1, stroke: blue),
  [c], [d],
  [e], grid.cell(stroke: aqua)[f]
)

---
// Test set rules on cells and folding
#set table.cell(stroke: 4pt)
#set table.cell(stroke: blue)
#set table.hline(stroke: red)
#set table.hline(stroke: 0.75pt)
#set table.vline(stroke: 0.75pt)
#set table.vline(stroke: aqua)

#table(
  columns: 3,
  gutter: 3pt,
  inset: 5pt,
  [a], [b], table.vline(position: end), [c],
  [d], [e], [f],
  table.hline(position: bottom),
  [g], [h], [i],
)

---
// Test stroke field on cell show rules
#set grid.cell(stroke: (x: 4pt))
#set grid.cell(stroke: (x: blue))
#show grid.cell: it => {
  test(it.stroke, (left: stroke(paint: blue, thickness: 4pt, dash: "loosely-dotted"), right: blue + 4pt, top: stroke(thickness: 1pt), bottom: none))
  it
}
#grid(
  stroke: (left: (dash: "loosely-dotted")),
  inset: 5pt,
  grid.hline(stroke: red),
  grid.cell(stroke: (top: 1pt))[a], grid.vline(stroke: yellow),
)

---
#table(
  columns: 3,
  [a], table.cell(colspan: 2)[b c],
  table.cell(stroke: blue)[d], [e], [f],
  [g], [h], table.cell(stroke: (left: yellow, top: green, right: aqua, bottom: red))[i],
  [j], [k], [l],
  table.cell(stroke: 3pt)[m], [n], table.cell(stroke: (dash: "loosely-dotted"))[o],
)

---
// Test per-column stroke array
#let t = table(
  columns: 3,
  stroke: (red, blue, green),
  [a], [b], [c],
  [d], [e], [f],
  [h], [i], [j],
)
#t
#set text(dir: rtl)
#t

---
#grid(
  columns: 3,
  inset: 3pt,
  stroke: (x, _) => (right: (5pt, (dash: "dotted")).at(calc.rem(x, 2)), bottom: (dash: "densely-dotted")),
  grid.vline(x: 0, stroke: red),
  grid.vline(x: 1, stroke: red),
  grid.vline(x: 2, stroke: red),
  grid.vline(x: 3, stroke: red),
  grid.hline(y: 0, end: 1, stroke: blue),
  grid.hline(y: 1, end: 1, stroke: blue),
  grid.cell[a],
  [b], [c]
)

---
#set page(height: 5em)
#table(
  columns: 3,
  inset: 3pt,
  table.hline(y: 0, end: none, stroke: 3pt + blue),
  table.vline(x: 0, end: none, stroke: 3pt + green),
  table.hline(y: 5, end: none, stroke: 3pt + red),
  table.vline(x: 3, end: none, stroke: 3pt + yellow),
  [a], [b], [c],
  [a], [b], [c],
  [a], [b], [c],
  [a], [b], [c],
  [a], [b], [c],
)

---
// Automatically positioned lines
// Plus stroke thickness ordering
#table(
  columns: 3,
  table.hline(stroke: red + 5pt),
  table.vline(stroke: blue + 5pt),
  table.vline(stroke: 2pt),
  [a],
  table.vline(x: 1, stroke: aqua + 5pt),
  [b],
  table.vline(stroke: aqua + 5pt),
  [c],
  table.vline(stroke: yellow + 5.2pt),
  table.hline(stroke: green + 5pt),
  [a], [b], [c],
  [a], table.hline(stroke: green + 2pt), table.vline(stroke: 2pt), [b], [c],
)

---
// Line specification order priority
// The last line should be blue, not red.
// The middle line should have disappeared.
#grid(
  columns: 2,
  inset: 2pt,
  grid.hline(y: 2, stroke: red + 5pt),
  grid.vline(),
  [a], [b],
  grid.hline(stroke: red),
  grid.hline(stroke: none),
  [c], grid.cell(stroke: (top: aqua))[d],
  grid.hline(stroke: blue),
)

---
// Position: bottom and position: end with gutter should have a visible effect
// of moving the lines after the next track.
#table(
  columns: 3,
  gutter: 3pt,
  stroke: blue,
  table.hline(end: 2, stroke: red),
  table.hline(end: 2, stroke: aqua, position: bottom),
  table.vline(end: 2, stroke: green), [a], table.vline(end: 2, stroke: green), table.vline(end: 2, position: end, stroke: orange), [b], table.vline(end: 2, stroke: aqua, position: end), table.vline(end: 2, stroke: green), [c], table.vline(end: 2, stroke: green),
  [d], [e], [f],
  table.hline(end: 2, stroke: red),
  [g], [h], [ie],
  table.hline(end: 2, stroke: green),
)

---
// Using position: bottom and position: end without gutter should be the same
// as placing a line after the next track.
#table(
  columns: 3,
  stroke: blue,
  table.hline(end: 2, stroke: red),
  table.hline(end: 2, stroke: aqua, position: bottom),
  table.vline(end: 2, stroke: green), [a], table.vline(end: 2, stroke: green), [b], table.vline(end: 2, stroke: aqua, position: end), table.vline(end: 2, stroke: green), [c], table.vline(end: 2, stroke: green),
  table.hline(end: 2, stroke: 5pt),
  [d], [e], [f],
  table.hline(end: 2, stroke: red),
  [g], [h], [i],
  table.hline(end: 2, stroke: red),
)

---
#table(
  columns: 3,
  gutter: 3pt,
  table.vline(stroke: red, position: end), [a], table.vline(stroke: red + 1.001pt), [b], [c],
  [d], [e], [f],
  table.hline(stroke: red),
  [g], [h], [i]
)

---
// Error: 5:3-5:32 cannot place horizontal line at invalid row
#table(
  [a],
  table.hline(stroke: aqua),
  table.hline(position: top),
  table.hline(position: bottom)
)

---
// Error: 6:3-6:32 cannot place horizontal line at invalid row
#table(
  gutter: 3pt,
  [a], table.vline(stroke: red),
  table.hline(stroke: aqua),
  table.hline(position: top),
  table.hline(position: bottom)
)

---
// Error: 3:3-3:31 line cannot end before it starts
#grid(
  columns: 3,
  grid.hline(start: 2, end: 1),
  [a], [b], [c],
)

---
// Error: 3:3-3:32 line cannot end before it starts
#table(
  columns: 3,
  table.vline(start: 2, end: 1),
  [a], [b], [c],
  [d], [e], [f],
  [g], [h], [i],
)

---
// Error: 24-31 expected `top` or `bottom`
#table.hline(position: horizon)

---
// Error: 24-30 expected `start` or `end`
#table.vline(position: center)

---
// Error: 24-28 expected `start` or `end`
#table.vline(position: left)

---
// Error: 24-29 expected `start` or `end`
#table.vline(position: right)

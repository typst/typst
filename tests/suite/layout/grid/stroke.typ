--- grid-stroke-tiling paged ---
#let double-line = tiling(size: (1.5pt, 1.5pt), {
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

--- grid-stroke-folding paged ---
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

--- grid-stroke-set-on-cell-and-line paged ---
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

--- grid-stroke-field-in-show paged ---
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

--- grid-stroke-complex paged ---
#table(
  columns: 3,
  [a], table.cell(colspan: 2)[b c],
  table.cell(stroke: blue)[d], [e], [f],
  [g], [h], table.cell(stroke: (left: yellow, top: green, right: aqua, bottom: red))[i],
  [j], [k], [l],
  table.cell(stroke: 3pt)[m], [n], table.cell(stroke: (dash: "loosely-dotted"))[o],
)

--- grid-stroke-array paged ---
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

--- grid-stroke-func paged ---
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

--- grid-stroke-manually-positioned-lines paged ---
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

--- grid-stroke-automatically-positioned-lines paged ---
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

--- grid-stroke-priority-line paged ---
// Line specification order priority
// The last line should be blue, not red.
// The middle aqua line should be gone due to the 'none' override.
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

--- grid-stroke-hline-position-bottom-gutter paged ---
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

--- grid-stroke-hline-position-bottom paged ---
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

--- grid-stroke-vline-position-left-and-right paged ---
// Test left and right for grid vlines.
#grid(
  columns: 3,
  inset: 5pt,
  grid.vline(stroke: green, position: left), grid.vline(stroke: red, position: right), [a],
  grid.vline(stroke: 2pt, position: left), grid.vline(stroke: red, position: right), [b],
  grid.vline(stroke: 2pt, position: left), grid.vline(stroke: red, position: right), [c],
  grid.vline(stroke: 2pt, position: left)
)

#grid(
  columns: 3,
  inset: 5pt,
  gutter: 3pt,
  grid.vline(stroke: green, position: left), grid.vline(stroke: red, position: right), [a],
  grid.vline(stroke: blue, position: left), grid.vline(stroke: red, position: right), [b],
  grid.vline(stroke: blue, position: left), grid.vline(stroke: red, position: right), [c],
  grid.vline(stroke: 2pt, position: left)
)

--- table-stroke-vline-position-left-and-right paged ---
// Test left and right for table vlines.
#table(
  columns: 3,
  inset: 5pt,
  table.vline(stroke: green, position: left), table.vline(stroke: red, position: right), [a],
  table.vline(stroke: 2pt, position: left), table.vline(stroke: red, position: right), [b],
  table.vline(stroke: 2pt, position: left), table.vline(stroke: red, position: right), [c],
  table.vline(stroke: 2pt, position: left)
)

#table(
  columns: 3,
  inset: 5pt,
  gutter: 3pt,
  table.vline(stroke: green, position: left), table.vline(stroke: red, position: right), [a],
  table.vline(stroke: blue, position: left), table.vline(stroke: red, position: right), [b],
  table.vline(stroke: blue, position: left), table.vline(stroke: red, position: right), [c],
  table.vline(stroke: 2pt, position: left)
)

--- grid-stroke-priority-line-cell paged ---
// Hlines and vlines should always appear on top of cell strokes.
#table(
  columns: 3,
  stroke: aqua,
  table.vline(stroke: red, position: end), [a], table.vline(stroke: red), [b], [c],
  table.cell(stroke: blue)[d], [e], [f],
  table.hline(stroke: red),
  [g], table.cell(stroke: blue)[h], [i],
)

#table(
  columns: 3,
  gutter: 3pt,
  stroke: aqua,
  table.vline(stroke: red, position: end), [a], table.vline(stroke: red), [b], [c],
  table.cell(stroke: blue)[d], [e], [f],
  table.hline(stroke: red),
  [g], table.cell(stroke: blue)[h], [i],
)

--- grid-stroke-priority-cell paged ---
// Ensure cell stroke overrides always appear on top.
#table(
  columns: 2,
  stroke: black,
  table.cell(stroke: red)[a], [b],
  [c], [d],
)

#table(
  columns: 2,
  table.cell(stroke: red)[a], [b],
  [c], [d],
)

--- grid-stroke-hline-position-bad paged ---
// Error: 7:3-7:32 cannot place horizontal line at the 'bottom' position of the bottom border (y = 2)
// Hint: 7:3-7:32 set the line's position to 'top' or place it at a smaller 'y' index
#table(
  columns: 2,
  [a], [b],
  [c], [d],
  table.hline(stroke: aqua),
  table.hline(position: top),
  table.hline(position: bottom)
)

--- grid-stroke-border-partial paged ---
// Test partial border line overrides
#set page(width: auto, height: 7em, margin: (bottom: 1em))
#table(
  columns: 4,
  stroke: (x, y) => if y == 0 or y == 4 { orange } else { aqua },
  table.hline(stroke: blue, start: 1, end: 2), table.cell(stroke: red, v(3em)), table.cell(stroke: blue)[b], table.cell(stroke: green)[c], [M],
  [a], [b], [c], [M],
  [d], [e], [f], [M],
  [g], [h], [i], [M],
  table.cell(stroke: red)[a], table.cell(stroke: blue)[b], table.cell(stroke: green)[c], [M],
  table.hline(stroke: blue, start: 1, end: 2),
)

--- grid-stroke-vline-colspan paged ---
// - Vline should be placed after the colspan.
// - Hline should be placed under the full-width rowspan.
#table(
  columns: 3,
  rows: 1.25em,
  inset: 1pt,
  stroke: none,
  table.cell(colspan: 2)[a], table.vline(stroke: red), table.hline(stroke: blue), [b],
  [c], [d], [e],
  table.cell(colspan: 3, rowspan: 2)[a], table.vline(stroke: blue), table.hline(stroke: red)
)

--- grid-stroke-hline-rowspan paged ---
// Red line should be above [c] (hline skips the shortest rowspan).
#set text(6pt)
#table(
  rows: 1em,
  columns: 2,
  inset: 1.5pt,
  table.cell(rowspan: 3)[a], table.cell(rowspan: 2)[b],
  table.hline(stroke: red),
  [c]
)

--- grid-stroke-hline-position-bottom-out-of-bounds paged ---
// Error: 8:3-8:32 cannot place horizontal line at the 'bottom' position of the bottom border (y = 2)
// Hint: 8:3-8:32 set the line's position to 'top' or place it at a smaller 'y' index
#table(
  columns: 2,
  gutter: 3pt,
  [a], [b],
  [c], [d], table.vline(stroke: red),
  table.hline(stroke: aqua),
  table.hline(position: top),
  table.hline(position: bottom)
)

--- grid-stroke-vline-position-bottom-out-of-bounds paged ---
// Error: 6:3-6:28 cannot place vertical line at the 'end' position of the end border (x = 2)
// Hint: 6:3-6:28 set the line's position to 'start' or place it at a smaller 'x' index
#grid(
  columns: 2,
  [a], [b],
  grid.vline(stroke: aqua),
  grid.vline(position: start),
  grid.vline(position: end)
)

--- grid-stroke-vline-position-bottom-out-of-bounds-gutter paged ---
// Error: 7:3-7:28 cannot place vertical line at the 'end' position of the end border (x = 2)
// Hint: 7:3-7:28 set the line's position to 'start' or place it at a smaller 'x' index
#grid(
  columns: 2,
  gutter: 3pt,
  [a], [b],
  grid.vline(stroke: aqua),
  grid.vline(position: start),
  grid.vline(position: end)
)

--- grid-stroke-hline-out-of-bounds paged ---
// Error: 4:3-4:19 cannot place horizontal line at invalid row 3
#grid(
  [a],
  [b],
  grid.hline(y: 3)
)

--- grid-stroke-hline-out-of-bounds-gutter paged ---
// Error: 5:3-5:19 cannot place horizontal line at invalid row 3
#grid(
  gutter: 3pt,
  [a],
  [b],
  grid.hline(y: 3)
)

--- grid-stroke-vline-out-of-bounds paged ---
// Error: 4:3-4:20 cannot place vertical line at invalid column 3
#table(
  columns: 2,
  [a], [b],
  table.vline(x: 3)
)

--- grid-stroke-vline-out-of-bounds-gutter paged ---
// Error: 5:3-5:20 cannot place vertical line at invalid column 3
#table(
  columns: 2,
  gutter: 3pt,
  [a], [b],
  table.vline(x: 3)
)

--- table-hline-in-grid paged ---
// Error: 7-20 cannot use `table.hline` as a grid line
// Hint: 7-20 use `grid.hline` instead
#grid(table.hline())

--- table-vline-in-grid paged ---
// Error: 7-20 cannot use `table.vline` as a grid line
// Hint: 7-20 use `grid.vline` instead
#grid(table.vline())

--- grid-hline-in-table paged ---
// Error: 8-20 cannot use `grid.hline` as a table line
// Hint: 8-20 use `table.hline` instead
#table(grid.hline())

--- grid-vline-in-table paged ---
// Error: 8-20 cannot use `grid.vline` as a table line
// Hint: 8-20 use `table.vline` instead
#table(grid.vline())

--- grid-hline-end-before-start-1 paged ---
// Error: 3:3-3:31 line cannot end before it starts
#grid(
  columns: 3,
  grid.hline(start: 2, end: 1),
  [a], [b], [c],
)

--- grid-hline-end-before-start-2 paged ---
// Error: 3:3-3:32 line cannot end before it starts
#table(
  columns: 3,
  table.vline(start: 2, end: 1),
  [a], [b], [c],
  [d], [e], [f],
  [g], [h], [i],
)

--- grid-hline-position-horizon paged ---
// Error: 24-31 expected `top` or `bottom`, found horizon
#table.hline(position: horizon)

--- grid-vline-position-center paged ---
// Error: 24-30 expected `start`, `left`, `right`, or `end`, found center
#table.vline(position: center)

--- grid-hline-position-right paged ---
// Error: 24-29 expected `top` or `bottom`, found right
#table.hline(position: right)

--- grid-vline-position-top paged ---
// Error: 24-27 expected `start`, `left`, `right`, or `end`, found top
#table.vline(position: top)

--- issue-7398-grid-line-end-oob paged ---
#set page(width: auto)
#table(
  columns: 2,
  [A], [B],
  [C], [D],
  table.vline(end: 3),
  table.hline(end: 3),
)

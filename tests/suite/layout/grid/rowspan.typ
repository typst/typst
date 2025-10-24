--- grid-rowspan render ---
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

--- grid-rowspan-gutter render ---
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

--- grid-rowspan-fixed-size render ---
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

--- grid-rowspan-cell-coordinates render ---
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

--- grid-rowspan-over-auto-row render ---
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

--- grid-rowspan-excessive render ---
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

--- grid-rowspan-excessive-gutter render ---
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

--- grid-rowspan-over-fr-row-at-end render ---
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

--- grid-rowspan-over-fr-row-at-start render ---
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

--- grid-rowspan-cell-order render ---
// Cell order
#let count = counter("count")
#show grid.cell: it => {
  count.step()
  context count.display()
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

--- grid-rowspan-unbreakable-1 render ---
#table(
    columns: 3,
    rows: (auto, auto, auto, 2em),
    gutter: 3pt,
    table.cell(rowspan: 4)[a \ b\ c\ d\ e], [c], [d],
    [e], table.cell(breakable: false, rowspan: 2)[f],
    [g]
)

--- grid-rowspan-unbreakable-2 render ---
// Test cell breakability
#show grid.cell: it => {
  test(it.breakable, (it.x, it.y) != (0, 6) and (it.y in (2, 5, 6) or (it.x, it.y) in ((0, 1), (2, 3), (1, 7))))
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

--- grid-rowspan-in-all-columns-stroke render ---
#table(
  columns: 2,
  table.cell(stroke: (bottom: red))[a], [b],
  table.hline(stroke: green),
  table.cell(stroke: (top: yellow, left: green, right: aqua, bottom: blue), colspan: 1, rowspan: 2)[d], table.cell(colspan: 1, rowspan: 2)[e],
  [f],
  [g]
)

--- grid-rowspan-in-all-columns-stroke-gutter render ---
#table(
  columns: 2,
  gutter: 3pt,
  table.cell(stroke: (bottom: red))[a], [b],
  table.hline(stroke: green),
  table.cell(stroke: (top: yellow, left: green, right: aqua, bottom: blue), colspan: 1, rowspan: 2)[d], table.cell(colspan: 1, rowspan: 2)[e],
  [f],
  [g]
)

--- grid-rowspan-block-full-height render ---
// Block below shouldn't expand to the end of the page, but stay within its
// rows' boundaries.
#set page(height: 9em)
#table(
  rows: (1em, 1em, 1fr, 1fr, auto),
  table.cell(rowspan: 2, block(width: 2em, height: 100%, fill: red)),
  table.cell(rowspan: 2, block(width: 2em, height: 100%, fill: red)),
  [a]
)

--- grid-rowspan-block-overflow render ---
#set page(height: 7em)
#table(
  columns: 3,
  [], [], table.cell(breakable: true, rowspan: 2, block(width: 2em, height: 100%, fill: red)),
  table.cell(breakable: false, block(width: 2em, height: 100%, fill: red)),
  table.cell(breakable: false, rowspan: 2, block(width: 2em, height: 100%, fill: red)),
)

// Rowspan split tests

--- grid-rowspan-split-1 render ---
#set page(height: 10em)
#table(
  columns: 2,
  rows: (auto, auto, 3em),
  fill: red,
  [a], table.cell(rowspan: 3, block(width: 50%, height: 10em, fill: orange) + place(bottom)[*ZD*]),
  [e],
  [f]
)

--- grid-rowspan-split-2 render ---
#set page(height: 10em)
#table(
  columns: 2,
  rows: (auto, auto, 3em),
  row-gutter: 1em,
  fill: red,
  [a], table.cell(rowspan: 3, block(width: 50%, height: 10em, fill: orange) + place(bottom)[*ZD*]),
  [e],
  [f]
)

--- grid-rowspan-split-3 render ---
#set page(height: 5em)
#table(
  columns: 2,
  fill: red,
  inset: 0pt,
  table.cell(fill: orange, rowspan: 10, place(bottom)[*Z*] + [x\ ] * 10 + place(bottom)[*ZZ*]),
  ..([y],) * 10,
  [a], [b],
)

--- grid-rowspan-split-4 render ---
#set page(height: 5em)
#table(
  columns: 2,
  fill: red,
  inset: 0pt,
  gutter: 2pt,
  table.cell(fill: orange, rowspan: 10, place(bottom)[*Z*] + [x\ ] * 10 + place(bottom)[*ZZ*]),
  ..([y],) * 10,
  [a], [b],
)

--- grid-rowspan-split-5 render ---
#set page(height: 5em)
#table(
  columns: 2,
  fill: red,
  inset: 0pt,
  table.cell(fill: orange, rowspan: 10, breakable: false, place(bottom)[*Z*] + [x\ ] * 10 + place(bottom)[*ZZ*]),
  ..([y],) * 10,
  [a], [b],
)

--- grid-rowspan-split-6 render ---
#set page(height: 5em)
#table(
  columns: 2,
  fill: red,
  inset: 0pt,
  gutter: 2pt,
  table.cell(fill: orange, rowspan: 10, breakable: false, place(bottom)[*Z*] + [x\ ] * 10 + place(bottom)[*ZZ*]),
  ..([y],) * 10,
  [a], [b],
)

--- grid-rowspan-split-7 render ---
#set page(height: 5em)
#grid(
  columns: 2,
  stroke: red,
  inset: 5pt,
  grid.cell(rowspan: 5)[a\ b\ c\ d\ e]
)

--- grid-rowspan-split-8 render ---
#set page(height: 5em)
#table(
  columns: 2,
  gutter: 3pt,
  stroke: red,
  inset: 5pt,
  table.cell(rowspan: 5)[a\ b\ c\ d\ e]
)

// Rowspan split without ending at the auto row

--- grid-rowspan-split-9 render ---
#set page(height: 6em)
#table(
  rows: (4em,) * 7 + (auto,) + (4em,) * 7,
  columns: 2,
  column-gutter: 1em,
  row-gutter: (1em, 2em) * 4,
  fill: (x, y) => if calc.odd(x + y) { orange.lighten(20%) } else { red },
  table.cell(rowspan: 15, [a \ ] * 15),
  [] * 15
)

--- grid-rowspan-split-10 render ---
#set page(height: 6em)
#table(
  rows: (4em,) * 7 + (auto,) + (4em,) * 7,
  columns: 2,
  column-gutter: 1em,
  row-gutter: (1em, 2em) * 4,
  fill: (x, y) => if calc.odd(x + y) { green } else { green.darken(40%) },
  table.cell(rowspan: 15, block(fill: blue, width: 2em, height: 4em * 14 + 3em)),
  [] * 15
)

--- grid-rowspan-split-11 render ---
#set page(height: 6em)
#table(
  rows: (3em,) * 15,
  columns: 2,
  column-gutter: 1em,
  row-gutter: (1em, 2em) * 4,
  fill: (x, y) => if calc.odd(x + y) { aqua } else { blue },
  table.cell(breakable: true, rowspan: 15, [a \ ] * 15),
  [] * 15
)

// Some splitting corner cases

--- grid-rowspan-split-12 render ---
// Inside the larger rowspan's range, there's an unbreakable rowspan and a
// breakable rowspan. This should work normally.
// The auto row will also expand ignoring the last fractional row.
#set page(height: 10em)
#table(
    gutter: 0.5em,
    columns: 2,
    rows: (2em,) * 10 + (auto, auto, 2em, 1fr),
    fill: (_, y) => if calc.even(y) { aqua } else { blue },
    table.cell(rowspan: 14, block(width: 2em, height: 2em * 10 + 2em + 5em, fill: red)[]),
    ..([a],) * 5,
    table.cell(rowspan: 3)[a\ b],
    table.cell(rowspan: 5, [a\ b\ c\ d\ e\ f\ g\ h]),
    [z]
)

--- grid-rowspan-split-13 render ---
// Inset moving to next region bug
#set page(width: 10cm, height: 2.5cm, margin: 0.5cm)
#set text(size: 11pt)
#table(
  columns: (1fr, 1fr, 1fr),
  [A],
  [B],
  [C],
  [D],
  table.cell(rowspan: 2, lorem(4)),
  [E],
  [F],
  [G],
)

--- grid-rowspan-split-14 render ---
// Second lorem must be sent to the next page, too big
#set page(width: 10cm, height: 9cm, margin: 1cm)
#set text(size: 11pt)
#table(
  columns: (1fr, 1fr, 1fr),
  align: center,
  rows: (4cm, auto),
  [A], [B], [C],
  table.cell(rowspan: 4, breakable: false, lorem(10)),
  [D],
  table.cell(rowspan: 2, breakable: false, lorem(20)),
  [E],
)

--- grid-rowspan-split-15 render ---
// Auto row must expand properly in both cases
#set text(10pt)
#show table.cell: it => if it.x == 0 { it } else { layout(size => size.height) }
#table(
  columns: 2,
  rows: (1em, auto, 2em, 3em, 4em),
  gutter: 3pt,
  table.cell(rowspan: 5, block(fill: orange, height: 15em)[a]),
  [b],
  [c],
  [d],
  [e],
  [f]
)

#table(
  columns: 2,
  rows: (1em, auto, 2em, 3em, 4em),
  gutter: 3pt,
  table.cell(rowspan: 5, breakable: false, block(fill: orange, height: 15em)[a]),
  [b],
  [c],
  [d],
  [e],
  [f]
)

--- grid-rowspan-split-16 render ---
// Expanding on unbreakable auto row
#set page(height: 7em, margin: (bottom: 2em))
#grid(
  columns: 2,
  rows: (1em, 1em, auto, 1em, 1em, 1em),
  fill: (x, y) => if x == 0 { aqua } else { blue },
  stroke: black,
  gutter: 2pt,
  grid.cell(rowspan: 5, block(height: 10em)[a]),
  [a],
  [b],
  grid.cell(breakable: false, v(3em) + [c]),
  [d],
  [e],
  [f], [g]
)

--- grid-rowspan-split-17 render ---
#show table.cell.where(x: 0): strong
#show table.cell.where(y: 0): strong
#set page(height: 13em)
#let lets-repeat(thing, n) = ((thing + colbreak(),) * (calc.max(0, n - 1)) + (thing,)).join()
#table(
  columns: 4,
  fill: (x, y) => if x == 0 or y == 0 { gray },
  [], [Test 1], [Test 2], [Test 3],
  table.cell(rowspan: 15, align: horizon, lets-repeat((rotate(-90deg, reflow: true)[*All Tests*]), 3)),
  ..([123], [456], [789]) * 15
)

--- issue-6399-grid-cell-rowspan-set-rule render ---
#set grid.cell(rowspan: 2)
#grid(columns: 2, [hehe])

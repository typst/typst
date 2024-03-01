// Some splitting corner cases

---
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

---
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

---
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

---
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

---
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

---
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

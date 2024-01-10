// Test cell positioning in grids.

---
#{
  show grid.cell: it => (it.x, it.y)
  grid(
    columns: 2,
    inset: 5pt,
    fill: aqua,
    [Hello], [World],
    [Sweet], [Italics]
  )
  grid(
    columns: 2,
    gutter: 3pt,
    [Hello], [World],
    [Sweet], [Italics]
  )
}

---
#grid(
  columns: 2,
  [A], [B],
  grid.cell(x: 1, y: 2)[C], grid.cell(x: 0, y: 2)[D],
  grid.cell(x: 1, y: 1)[E], grid.cell(x: 0, y: 1)[F],
)

---
#grid(
  columns: 3,
  rows: 1.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A],
  grid.cell(x: 2, y: 3)[B]
)

#table(
  columns: (3em, 1em, 3em),
  rows: 1.5em,
  inset: (top: 0pt, bottom: 0pt, rest: 5pt),
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  align: (x, y) => (left, center, right).at(x),
  [A],
  table.cell(x: 2, y: 3)[B]
)

---
// Error: 2-4:2 Attempted to place two different cells at column 0, row 0.
#grid(
  [A],
  grid.cell(x: 0, y: 0)[This shall error]
)

---
#grid(
  grid.cell(x: 0, y: 0)[This shall not error],
  [A]
)

---
#grid(
  columns: 3,
  rows: 1.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A], grid.cell(y: 1)[B], [C], grid.cell(x: auto, y: 1)[D], [E],
  grid.cell(y: 2)[F], grid.cell(x: 0)[G], grid.cell(x: 0, y: auto)[H],
  grid.cell(x: 1)[I]
)

#table(
  columns: 3,
  rows: 1.5em,
  inset: 5pt,
  fill: (x, y) => if (x, y) == (0, 0) { blue } else if (x, y) == (2, 3) { red } else { green },
  [A], table.cell(y: 1)[B], [C], table.cell(x: auto, y: 1)[D], [E],
  table.cell(y: 2)[F], table.cell(x: 0)[G], table.cell(x: 0, y: auto)[H],
  table.cell(x: 1)[I]
)

---
// Doc example 1
#set page(width: auto)
#show grid.cell: it => {
  if it.y == 0 {
    set text(white)
    strong(it)
  } else {
    // For the second row and beyond, we will write the day number for each
    // cell.

    // In general, a cell's index is given by cell.x + columns * cell.y.
    // Days start in the second grid row, so we subtract 1 row.
    // But the first day is day 1, not day 0, so we add 1.
    let day = it.x + 7 * (it.y - 1) + 1
    if day <= 31 {
      // Place the day's number at the top left of the cell.
      // Only if the day is valid for this month (not 32 or higher).
      place(top + left, dx: 2pt, dy: 2pt, text(8pt, red.darken(40%))[#day])
    }
    it
  }
}

#grid(
  fill: (x, y) => if y == 0 { gray.darken(50%) },
  columns: (30pt,) * 7,
  rows: (auto, 30pt),
  // Events will be written at the bottom of each day square.
  align: bottom,
  inset: 5pt,
  stroke: (thickness: 0.5pt, dash: "densely-dotted"),

  [Sun], [Mon], [Tue], [Wed], [Thu], [Fri], [Sat],

  // This event will occur on the first Friday (sixth column).
  grid.cell(x: 5, fill: yellow.darken(10%))[Call],

  // This event will occur every Monday (second column).
  // We have to repeat it 5 times so it occurs every week.
  ..(grid.cell(x: 1, fill: red.lighten(50%))[Meet],) * 5,

  // This event will occur at day 19.
  grid.cell(x: 4, y: 3, fill: orange.lighten(25%))[Talk],

  // These events will occur at the second week, where available.
  grid.cell(y: 2, fill: aqua)[Chat],
  grid.cell(y: 2, fill: aqua)[Walk],
)

---
// Doc example 2
#set page(width: auto)
#show table.cell: it => {
  if it.x == 0 or it.y == 0 {
    set text(white)
    strong(it)
  } else if it.body == [] {
    // Replace empty cells with 'N/A'
    pad(rest: it.inset)[_N/A_]
  } else {
    it
  }
}

#table(
  fill: (x, y) => if x == 0 or y == 0 { gray.darken(50%) },
  columns: 4,
  [], [Exam 1], [Exam 2], [Exam 3],
  ..([John], [Mary], [Jake], [Robert]).map(table.cell.with(x: 0)),

  // Mary got grade A on Exam 3.
  table.cell(x: 3, y: 2, fill: green)[A],

  // Everyone got grade A on Exam 2.
  ..(table.cell(x: 2, fill: green)[A],) * 4,

  // Robert got grade B on other exams.
  ..(table.cell(y: 4, fill: aqua)[B],) * 2,
)

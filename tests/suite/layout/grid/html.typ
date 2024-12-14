--- basic-table html ---
#table(
  columns: 3,
  rows: 3,

  table.header(
    [The],
    [first],
    [and],
    [the],
    [second],
    [row],
    table.hline(stroke: red)
  ),

  table.cell(x: 1, rowspan: 2)[Baz],
  [Foo],
  [Bar],

  [1],
  // Baz spans into the next cell
  [2],

  table.cell(colspan: 2)[3],
  [4],

  table.footer(
    [The],
    [last],
    [row],
  ),
)

--- col-gutter-table html ---
#table(
  columns: 3,
  column-gutter: 3pt,
  [a], [b], [c],
  [d], [e], [f],
  [g], [h], [i]
)

--- row-gutter-table html ---
#table(
  columns: 3,
  row-gutter: 3pt,
  [a], [b], [c],
  [d], [e], [f],
  [g], [h], [i]
)

--- col-row-gutter-table html ---
#table(
  columns: 3,
  gutter: 3pt,
  [a], [b], [c],
  [d], [e], [f],
  [g], [h], [i]
)

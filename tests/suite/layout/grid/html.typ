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

--- multi-header-table html ---
#table(
  columns: 2,

  table.header(
    [First], [Header]
  ),
  table.header(
    [Second], [Header]
  ),
  table.header(
    [Level 2], [Header],
    level: 2,
  ),
  table.header(
    [Level 3], [Header],
    level: 3,
  ),

  [Body], [Cells],
  [Yet], [More],

  table.footer(
    [Footer], [Row],
    [Ending], [Table],
  ),
)

--- multi-header-inside-table html ---
#table(
  columns: 2,

  table.header(
    [First], [Header]
  ),
  table.header(
    [Second], [Header]
  ),
  table.header(
    [Level 2], [Header],
    level: 2,
  ),
  table.header(
    [Level 3], [Header],
    level: 3,
  ),

  [Body], [Cells],
  [Yet], [More],

  table.header(
    [Level 2], [Header Inside],
    level: 2,
  ),
  table.header(
    [Level 3],
    level: 3,
  ),

  [Even], [More],
  [Body], [Cells],

  table.header(
    [One Last Header],
    [For Good Measure],
    repeat: false,
    level: 4,
  ),

  table.footer(
    [Footer], [Row],
    [Ending], [Table],
  ),
)

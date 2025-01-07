--- basic-table html ---
#table(
  columns: 3,
  rows: 3,

  table.header(
  [The],
  [first],
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
)

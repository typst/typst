--- grid-subheaders-basic ---
#grid(
  grid.header(
    [a]
  ),
  grid.header(
    level: 2,
    [b]
  ),
  [c]
)

--- grid-subheaders-basic-non-consecutive ---
#grid(
  grid.header(
    [a]
  ),
  [x],
  grid.header(
    level: 2,
    [b]
  ),
  [y],
)

--- grid-subheaders-basic-replace ---
#grid(
  grid.header(
    [a]
  ),
  [x],
  grid.header(
    level: 2,
    [b]
  ),
  [y],
  grid.header(
    level: 2,
    [c]
  ),
  [z],
)

--- grid-subheaders-repeat ---
#set page(height: 8em)
#grid(
  grid.header(
    [a]
  ),
  grid.header(
    level: 2,
    [b]
  ),
  ..([c],) * 10,
)

--- grid-subheaders-repeat-non-consecutive ---
#set page(height: 8em)
#grid(
  grid.header(
    [a]
  ),
  [x],
  grid.header(
    level: 2,
    [b]
  ),
  ..([y],) * 10,
)

--- grid-subheaders-repeat-replace ---
#set page(height: 8em)
#grid(
  grid.header(
    [a]
  ),
  [x],
  grid.header(
    level: 2,
    [b]
  ),
  ..([y],) * 10,
  grid.header(
    level: 2,
    [c]
  ),
  ..([z],) * 10,
)

--- grid-subheaders ---
#set page(width: auto, height: 12em)
#let rows(n) = {
  range(n).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789], [?], table.hline(start: 4, end: 5, stroke: red))).flatten()
}
#table(
  columns: 5,
  align: center + horizon,
  table.header(
    table.cell(colspan: 5)[*Cool Zone*],
  ),
  table.header(
    level: 2,
    table.cell(stroke: red)[*Name*], table.cell(stroke: aqua)[*Number*], [*Data 1*], [*Data 2*], [*Etc*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..rows(6),
  table.header(
    level: 2,
    table.cell(stroke: red)[*New Name*], table.cell(stroke: aqua, colspan: 4)[*Other Data*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..rows(5)
)

--- grid-subheaders-alone ---
#table(
  table.header(
    [a]
  ),
  table.header(
    level: 2,
    [b]
  ),
)

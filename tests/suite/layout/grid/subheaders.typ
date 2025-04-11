--- grid-subheaders-colorful ---
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
  ..rows(2),
  table.header(
    level: 2,
    table.cell(stroke: red)[*New Name*], table.cell(stroke: aqua, colspan: 4)[*Other Data*],
    table.hline(start: 2, end: 3, stroke: yellow)
  ),
  ..rows(3)
)

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

--- grid-subheaders-repeat-replace-multiple-levels ---
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
  grid.header(
    level: 3,
    [c]
  ),
  ..([y],) * 10,
  grid.header(
    level: 2,
    [d]
  ),
  ..([z],) * 6,
)

--- grid-subheaders-repeat-replace-orphan ---
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
  ..([y],) * 12,
  grid.header(
    level: 2,
    [c]
  ),
  ..([z],) * 10,
)

--- grid-subheaders-repeat-replace-double-orphan ---
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
  ..([y],) * 11,
  grid.header(
    level: 2,
    [c]
  ),
  grid.header(
    level: 3,
    [d]
  ),
  ..([z],) * 10,
)

--- grid-subheaders-repeat-replace-didnt-fit-once ---
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
    [c\ c\ c]
  ),
  ..([z],) * 4,
)

--- grid-subheaders-multi-page-row ---
#set page(height: 8em)
#grid(
  columns: 2,
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
    level: 3,
    [c]
  ),
  [a], [b],
  grid.cell(
    block(fill: red, width: 1.5em, height: 6.4em)
  ),
  [y],
  ..([z],) * 10,
)

--- grid-subheaders-multi-page-rowspan ---
#set page(height: 8em)
#grid(
  columns: 2,
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
    level: 3,
    [c]
  ),
  [z], [z],
  grid.cell(
    rowspan: 5,
    block(fill: red, width: 1.5em, height: 6.4em)
  ),
  [cell],
  [cell]
)

--- grid-subheaders-multi-page-row-right-after ---
#set page(height: 8em)
#grid(
  columns: 1,
  grid.header(
    [a]
  ),
  [x],
  grid.header(
    level: 2,
    [b]
  ),
  grid.header(
    level: 3,
    [c]
  ),
  grid.cell(
    block(fill: red, width: 1.5em, height: 6.4em)
  ),
  [done.],
  [done.]
)

--- grid-subheaders-multi-page-rowspan-right-after ---
#set page(height: 8em)
#grid(
  columns: 2,
  grid.header(
    [a]
  ),
  [x], [y],
  grid.header(
    level: 2,
    [b]
  ),
  grid.header(
    level: 3,
    [c]
  ),
  grid.cell(
    rowspan: 5,
    block(fill: red, width: 1.5em, height: 6.4em)
  ),
  [cell],
  [cell],
  grid.cell(x: 0)[done.],
  grid.cell(x: 0)[done.]
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

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

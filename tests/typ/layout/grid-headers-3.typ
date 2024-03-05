// Test lack of space for header + text.
#set page(height: 9em)

#table(
  rows: (auto, 2.5em, auto, auto, 10em),
  gutter: 3pt,
  table.header(
    [*Hello*],
    [*World*]
  ),
  table.cell(rowspan: 3, lorem(80))
)

---
// Orphan header prevention test
#set page(height: 12em)
#v(8em)
#grid(
  columns: 3,
  grid.header(
    [*Mui*], [*A*], grid.cell(rowspan: 2, fill: orange)[*B*],
    [*Header*], [*Header* #v(0.1em)]
  ),
  ..([Test], [Test], [Test]) * 20
)

---
// Empty header should just be a repeated blank row
#set page(height: 12em)
#table(
  columns: 4,
  align: center + horizon,
  table.header(),
  ..range(0, 4).map(i => ([John \##i], table.cell(stroke: green)[123], table.cell(stroke: blue)[456], [789])).flatten()
)

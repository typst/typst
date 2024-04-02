#set page(height: 17em)
#table(
  rows: (auto, 2.5em, auto),
  table.header[*Hello*][*World*],
  block(width: 2em, height: 10em, fill: red),
  table.footer[*Bye*][*World*],
)

---
// Rowspan sizing algorithm doesn't do the best job at non-contiguous content
// ATM.
#set page(height: 20em)

#table(
  rows: (auto, 2.5em, 2em, auto, 5em, 2em, 2.5em),
  table.header[*Hello*][*World*],
  table.cell(rowspan: 3, lorem(20)),
  table.footer[*Ok*][*Bye*],
)

---
// This should look right
#set page(height: 20em)

#table(
  rows: (auto, 2.5em, 2em, auto),
  gutter: 3pt,
  table.header[*Hello*][*World*],
  table.cell(rowspan: 3, lorem(20)),
  table.footer[*Ok*][*Bye*],
)

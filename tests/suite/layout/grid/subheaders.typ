--- grid-subheaders-demo ---
#set page(height: 15.2em)
#table(
  columns: 2,
  align: center,
  table.header(
    table.cell(colspan: 2)[*Regional User Data*],
  ),
  table.header(
    level: 2,
    table.cell(colspan: 2)[*Germany*],
    [*Username*], [*Joined*]
  ),
  [john123], [2024],
  [rob8], [2025],
  [joe1], [2025],
  [joe2], [2025],
  [martha], [2025],
  [pear], [2025],
  table.header(
    level: 2,
    table.cell(colspan: 2)[*United States*],
    [*Username*], [*Joined*]
  ),
  [cool4], [2023],
  [roger], [2023],
  [bigfan55], [2022]
)

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

--- grid-subheaders-basic-with-footer ---
#grid(
  grid.header(
    [a]
  ),
  grid.header(
    level: 2,
    [b]
  ),
  [c],
  grid.footer(
    [d]
  )
)

--- grid-subheaders-basic-non-consecutive-with-footer ---
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
  grid.footer(
    [f]
  )
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

--- grid-subheaders-repeat-with-footer ---
#set page(height: 8em)
#grid(
  grid.header(
    [a]
  ),
  [m],
  grid.header(
    level: 2,
    [b]
  ),
  ..([c],) * 10,
  grid.footer(
    [f]
  )
)

--- grid-subheaders-repeat-gutter ---
// Gutter below the header is also repeated
#set page(height: 8em)
#grid(
  inset: (bottom: 0.5pt),
  stroke: (bottom: 1pt),
  gutter: (1pt, 6pt, 1pt),
  grid.header(
    [a]
  ),
  grid.header(
    level: 2,
    [b]
  ),
  ..([c],) * 10,
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

--- grid-subheaders-repeat-replace-with-footer ---
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
  grid.footer(
    [f]
  )
)

--- grid-subheaders-repeat-replace-with-footer-orphan ---
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
  grid.footer(
    [f]
  )
)

--- grid-subheaders-repeat-replace-short-lived ---
// No orphan prevention for short-lived headers
// (followed by replacing headers).
#set page(height: 8em)
#grid(
  grid.header(
    [a]
  ),
  grid.header(
    level: 2,
    [b]
  ),
  grid.header(
    level: 2,
    [c]
  ),
  grid.header(
    level: 2,
    [d]
  ),
  grid.header(
    level: 2,
    [e]
  ),
  grid.header(
    level: 2,
    [f]
  ),
  grid.header(
    level: 2,
    [g]
  ),
  grid.header(
    level: 2,
    [h]
  ),
  grid.header(
    level: 2,
    [i]
  ),
  grid.header(
    level: 2,
    [j]
  ),
  grid.header(
    level: 3,
    [k]
  ),
  ..([z],) * 10,
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

--- grid-subheaders-non-repeat ---
#set page(height: 8em)
#grid(
  grid.header(
    [a],
    repeat: false,
  ),
  [x],
  grid.header(
    level: 2,
    repeat: false,
    [b]
  ),
  ..([y],) * 10,
)

--- grid-subheaders-non-repeat-replace ---
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
  ..([y],) * 9,
  grid.header(
    level: 2,
    [d],
    repeat: false,
  ),
  ..([z],) * 6,
)

--- grid-subheaders-non-repeating-replace-orphan ---
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
    repeat: false,
    [c]
  ),
  ..([z],) * 10,
)

--- grid-subheaders-non-repeating-replace-didnt-fit-once ---
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
    repeat: false,
    [c\ c\ c]
  ),
  ..([z],) * 4,
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

--- grid-subheaders-multi-page-row-with-footer ---
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
  grid.footer(
    [f]
  )
)

--- grid-subheaders-multi-page-rowspan-with-footer ---
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
  [cell],
  grid.footer(
    [f]
  )
)

--- grid-subheaders-multi-page-row-right-after-with-footer ---
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
  [done.],
  grid.footer(
    [f]
  )
)

--- grid-subheaders-multi-page-rowspan-gutter ---
#set page(height: 9em)
#grid(
  columns: 2,
  column-gutter: 4pt,
  row-gutter: (0pt, 4pt, 8pt, 4pt),
  inset: (bottom: 0.5pt),
  stroke: (bottom: 1pt),
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
  [cell],
  [a\ b],
  grid.cell(x: 0)[end],
)

--- grid-subheaders-non-repeating-header-before-multi-page-row ---
#set page(height: 6em)
#grid(
  grid.header(
    repeat: false,
    [h]
  ),
  [row #colbreak() row]
)


--- grid-subheaders-short-lived-no-orphan-prevention ---
// No orphan prevention for short-lived headers.
#set page(height: 8em)
#v(5em)
#grid(
  grid.header(
    level: 2,
    [b]
  ),
  grid.header(
    level: 2,
    [c]
  ),
  [d]
)

--- grid-subheaders-repeating-orphan-prevention ---
#set page(height: 8em)
#v(4.5em)
#grid(
  grid.header(
    repeat: true,
    level: 2,
    [L2]
  ),
  grid.header(
    repeat: true,
    level: 4,
    [L4]
  ),
  [a]
)

--- grid-subheaders-non-repeating-orphan-prevention ---
#set page(height: 8em)
#v(4.5em)
#grid(
  grid.header(
    repeat: false,
    level: 2,
    [L2]
  ),
  grid.header(
    repeat: false,
    level: 4,
    [L4]
  ),
  [a]
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

--- grid-subheaders-alone-no-orphan-prevention ---
#set page(height: 5.3em)
#v(2em)
#grid(
  grid.header(
  // (
    [L1]
  ),
  grid.header(
  // (
    level: 2,
    [L2]
  ),
)

--- grid-subheaders-alone-with-footer ---
#table(
  table.header(
    [a]
  ),
  table.header(
    level: 2,
    [b]
  ),
  table.footer(
    [c],
  )
)

--- grid-subheaders-alone-with-footer-no-orphan-prevention ---
#set page(height: 5.3em)
#table(
  table.header(
    [L1]
  ),
  table.header(
    level: 2,
    [L2]
  ),
  table.footer(
    [a],
  )
)

--- grid-subheaders-alone-with-gutter-and-footer-no-orphan-prevention ---
#set page(height: 5.5em)
#table(
  gutter: 4pt,
  table.header(
    [L1]
  ),
  table.header(
    level: 2,
    [L2]
  ),
  table.footer(
    [a],
  )
)

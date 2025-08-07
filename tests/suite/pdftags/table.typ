--- table-tags-basic pdftags ---
#table(
  columns: 3,
  table.header([H1], [H2], [H3]),
  [a1], [a2], [a3],
  [b1], [b2], [b3],
)

--- table-tags-column-and-row-header pdftags ---
#table(
  columns: 3,
  table.header([H1], [H2], [H3]),
  pdf.header-cell(scope: "row")[10:00], [a2], [a3],
  pdf.header-cell(scope: "row")[12:30], [b2], [b3],
)

--- table-tags-missing-cells pdftags ---
#table(
  columns: 3,
  table.header(level: 1, [H1], [H1], [H1]),
  table.header(level: 2, [H2], [H2], [H2]),

  // the middle cell is missing
  table.cell(x: 0)[],
  table.cell(x: 2)[],

  // the last cell is missing, its type should be inferred from the row
  table.header(level: 2, [H2], [H2]),

  // last cell is missing
  [], [],

  table.footer(
    table.cell(x: 1)[F],
    table.cell(x: 2)[F],
  ),
)

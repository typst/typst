--- grid-subfooters-demo ---
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
  table.footer(
    level: 2,
    [*Mode*], [2025],
    table.cell(colspan: 2)[*Totals*],
  ),
  // TODO: Why does it overflow here?
  table.header(
    level: 2,
    table.cell(colspan: 2)[*United States*],
    [*Username*], [*Joined*]
  ),
  [cool4], [2023],
  [roger], [2023],
  [bigfan55], [2022],
  table.footer(
    level: 2,
    [*Mode*], [2023],
    table.cell(colspan: 2)[*Totals*],
  ),
  table.footer(
    table.cell(colspan: 2)[*Data Inc.*],
  ),
)

--- grid-subfooters-basic ---
#grid(
  [a],
  grid.footer(level: 2, [b]),
  grid.footer([c]),
)

--- grid-subfooters-basic-non-consecutive ---
#grid(
  [x],
  grid.footer(level: 2, [a]),
  [y],
  grid.footer([b]),
)

--- grid-subfooters-basic-replace ---
#grid(
  [x],
  grid.footer(level: 2, [a]),
  [y],
  grid.footer(level: 2, [b]),
  [z],
  grid.footer([c]),
)

--- grid-subfooters-basic-with-header ---
#grid(
  grid.header([a]),
  [b],
  grid.footer(level: 2, [c]),
  grid.footer([d]),
)

--- grid-subfooters-basic-non-consecutive-with-header ---
#grid(
  grid.header([a]),
  [x],
  grid.footer(level: 2, [b]),
  [y],
  grid.footer([f])
)

--- grid-subfooters-repeat ---
#set page(height: 8em)
#grid(
  ..([a],) * 10,
  grid.footer(level: 2, [b]),
  grid.footer([c]),
)

--- grid-subfooters-repeat-non-consecutive ---
#set page(height: 8em)
#grid(
  ..([y],) * 10,
  grid.footer(level: 2, [b]),
  [x],
  grid.footer([a]),
)

--- grid-subfooters-repeat-with-header ---
#set page(height: 8em)
#grid(
  grid.header([a]),
  ..([b],) * 10,
  grid.footer(level: 2, [c]),
  [m],
  grid.footer([f])
)

--- grid-subfooters-repeat-gutter ---
// Gutter above the footer is also repeated
#set page(height: 8em)
#grid(
  inset: (top: 0.5pt),
  stroke: (top: 1pt),
  gutter: (1pt,) * 9 + (6pt, 1pt),
  ..([a],) * 10,
  grid.footer(level: 2, [b]),
  grid.footer([c]),
)

--- grid-subfooters-repeat-replace ---
#set page(height: 8em)
#grid(
  ..([x],) * 10,
  grid.footer(level: 2, [a]),
  ..([y],) * 10,
  grid.footer(level: 2, [b]),
  [z],
  grid.footer([c]),
)

--- grid-subfooters-repeat-replace-multiple-levels ---
// TODO: This is overflowing
#set page(height: 8em)
#grid(
  ..([x],) * 6,
  grid.footer(level: 2, [a]),
  ..([y],) * 10,
  grid.footer(level: 3, [b]),
  grid.footer(level: 2, [c]),
  [z],
  grid.footer([d]),
)

--- grid-subfooters-repeat-replace-gutter ---
#set page(height: 8em)
#grid(
  gutter: 3pt,
  ..([x],) * 3,
  grid.footer(level: 2, [a]),
  ..([y],) * 8,
  grid.footer(level: 2, [b]),
  [z],
  grid.footer([c]),
)

--- grid-subfooters-repeat-replace-widow ---
#set page(height: 8em)
#grid(
  ..([x],) * 14,
  grid.footer(level: 2, [a]),
  ..([y],) * 8,
  grid.footer(level: 2, [b]),
  [z],
  grid.footer([c]),
)

--- grid-subfooters-repeat-replace-double-widow ---
#set page(height: 8em)
#grid(
  ..([x],) * 12,
  grid.footer(level: 3, [a]),
  grid.footer(level: 2, [b]),
  ..([y],) * 11,
  grid.footer(level: 2, [c]),
  [z],
  grid.footer([d]),
)

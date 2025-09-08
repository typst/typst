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

--- table-tags-explicit-lines pdftags ---
#table(
  columns: 2,
  [a], table.vline(stroke: green), [b],
  table.hline(stroke: red),
  [c], [d],
  table.hline(stroke: blue),
)

--- table-tags-unset-bottom-line pdftags ---
#table(
  columns: 2,
  [a], [b],
  [c], [d],
  table.hline(stroke: none),
)

--- table-tags-different-default-border pdftags ---
#table(
  columns: 2,
  stroke: red + 2pt,
  table.hline(stroke: black),
  [a], [b],
  [c], [d],
  [e], [f],
  table.hline(stroke: black),
)

--- table-tags-show-rule pdftags ---
// The table contents are transformed by a show rule.
#set table(columns: (10pt, auto))
#show table: it => it.columns
#table[A][B][C][D]

--- table-tags-rowspan-split-1 pdftags ---
#set page(height: 6em)
#table(
  rows: (4em, auto, 4em),
  columns: 2,
  table.cell(rowspan: 3, [a\ ] * 4),
  [b], [c], [d],
)

--- table-tags-rowspan-split-2 pdftags ---
#set page(height: 6em)
#table(
  rows: (4em, auto, 4em),
  columns: 3,
  [a1], table.cell(rowspan: 3, [b\ ] * 5), [c1],
  [a2], [c2],
  [a3], [c3],
)

--- table-tags-citation-in-repeated-header pdftags ---
// Error: 3:16-3:23 PDF/UA-1 error: PDF artifacts may not contain links
// Hint: 3:16-3:23 references, citations, and footnotes are also considered links in PDF
#set page(height: 60pt)
#table(
  table.header[@netwok],
  [A],
  [A],
)

#show bibliography: none
#bibliography("/assets/bib/works.bib")

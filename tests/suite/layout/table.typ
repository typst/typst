// Test tables.

--- table-empty render pdftags ---
#table()

--- table-newlines render ---
#set page(height: 70pt)
#set table(fill: (x, y) => if calc.even(x + y) { rgb("aaa") })

#table(
  columns: (1fr,) * 3,
  stroke: 2pt + rgb("333"),
  [A], [B], [C], [], [], [D \ E \ F \ \ \ G], [H],
)

--- table-fill-basic render ---
#table(columns: 3, stroke: none, fill: green, [A], [B], [C])

--- table-fill-bad render ---
// Error: 14-19 expected color, gradient, tiling, none, array, or function, found string
#table(fill: "hey")

--- table-align-array render ---
// Test alignment with array.
#table(
  columns: (1fr, 1fr, 1fr),
  align: (left, center, right),
  [A], [B], [C]
)

// Test empty array.
#set align(center)
#table(
  columns: (1fr, 1fr, 1fr),
  align: (),
  [A], [B], [C]
)

--- table-inset render ---
// Test inset.
#table(
  columns: 3,
  inset: 10pt,
  [A], [B], [C]
)

#table(
  columns: 3,
  inset: (y: 10pt),
  [A], [B], [C]
)

#table(
  columns: 3,
  inset: (left: 20pt, rest: 10pt),
  [A], [B], [C]
)

#table(
  columns: 2,
  inset: (
    left: 20pt,
    right: 5pt,
    top: 10pt,
    bottom: 3pt,
  ),
  [A],
  [B],
)

#table(
  columns: 3,
  fill: (x, y) => (if y == 0 { aqua } else { orange }).darken(x * 15%),
  inset: (x, y) => (left: if x == 0 { 0pt } else { 5pt }, right: if x == 0 { 5pt } else { 0pt }, y: if y == 0 { 0pt } else { 5pt }),
  [A], [B], [C],
  [A], [B], [C],
)

#table(
  columns: 3,
  inset: (0pt, 5pt, 10pt),
  fill: (x, _) => aqua.darken(x * 15%),
  [A], [B], [C],
)

--- table-inset-fold render ---
// Test inset folding
#set table(inset: 10pt)
#set table(inset: (left: 0pt))

#table(
  fill: red,
  inset: (right: 0pt),
  table.cell(inset: (top: 0pt))[a]
)

--- table-gutters render ---
// Test interaction with gutters.
#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  row-gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  column-gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

#table(
  columns: (3em, 3em),
  fill: (x, y) => (red, blue).at(calc.rem(x, 2)),
  align: (x, y) => (left, right).at(calc.rem(y, 2)),
  gutter: 5pt,
  [A], [B],
  [C], [D],
  [E], [F],
  [G], [H]
)

--- table-contextual-measurement render ---
// Test that table cells with varying contextual results are properly
// measured.
#let c = counter("c")
#let k = context square(width: c.get().first() * 5pt)
#let u(n) = [#n] + c.update(n)
#table(
  columns: 3,
  u(1), k, u(2),
  k, u(4), k,
  k, k, k,
)

--- table-header-citation render ---
#set page(height: 60pt)
#table(
  table.header[@netwok],
  [A],
  [A],
)

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- table-header-counter render ---
#set page(height: 60pt)
#let c = counter("c")
#table(
  table.header(c.step() + context c.display()),
  [A],
  [A],
)

--- table-header-footer-madness render ---
#set page(height: 100pt)
#let c = counter("c")
#let it = context c.get().first() * v(10pt)
#table(
  table.header(c.step()),
  [A],
  [A],
  [A],
  [A],
  [A],
  [A],
  [A],
  table.footer(it),
)

--- table-cell-override render ---
// Cell override
#table(
  align: left,
  fill: red,
  stroke: blue,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  table.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A],
  table.cell(align: horizon)[G2], [A\ A\ A],
  table.cell(inset: 0pt)[I], [F],
  [H], table.cell(fill: blue)[J]
)

--- table-cell-show render ---
// Cell show rule
#show table.cell: it => [Zz]

#table(
  align: left,
  fill: red,
  stroke: blue,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  table.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A]
)

--- table-cell-show-and-override render ---
#show table.cell: it => (it.align, it.fill)
#table(
  align: left,
  row-gutter: 5pt,
  [A],
  table.cell(align: right)[B],
  table.cell(fill: aqua)[B],
)

--- table-cell-set render ---
// Cell set rules
#set table.cell(align: center)
#show table.cell: it => (it.align, it.fill, it.inset)
#set table.cell(inset: 20pt)
#table(
  align: left,
  row-gutter: 5pt,
  [A],
  table.cell(align: right)[B],
  table.cell(fill: aqua)[B],
)

--- table-cell-folding render ---
// Test folding per-cell properties (align and inset)
#table(
  columns: (1fr, 1fr),
  rows: (2.5em, auto),
  align: right,
  fill: (x, y) => (green, aqua).at(calc.rem(x + y, 2)),
  [Top], table.cell(align: bottom)[Bot],
  table.cell(inset: (bottom: 0pt))[Bot], table.cell(inset: (bottom: 0pt))[Bot]
)

--- table-cell-align-override render ---
// Test overriding outside alignment
#set align(bottom + right)
#table(
  columns: (1fr, 1fr),
  rows: 2em,
  align: auto,
  fill: green,
  [BR], [BR],
  table.cell(align: left, fill: aqua)[BL], table.cell(align: top, fill: red.lighten(50%))[TR]
)

--- table-cell-various-overrides render ---
#table(
  columns: 2,
  fill: green,
  align: right,
  [*Name*], [*Data*],
  table.cell(fill: blue)[J.], [Organizer],
  table.cell(align: center)[K.], [Leader],
  [M.], table.cell(inset: 0pt)[Player]
)

--- table-cell-show-emph render ---
#{
  show table.cell: emph
  table(
    columns: 2,
    [Person], [Animal],
    [John], [Dog]
  )
}

--- table-cell-show-based-on-position render ---
// Style based on position
#{
  show table.cell: it => {
    if it.y == 0 {
      strong(it)
    } else if it.x == 1 {
      emph(it)
    } else {
      it
    }
  }
  table(
    columns: 3,
    gutter: 3pt,
    [Name], [Age], [Info],
    [John], [52], [Nice],
    [Mary], [50], [Cool],
    [Jake], [49], [Epic]
  )
}

--- table-cell-par render ---
// Ensure that table cells aren't considered paragraphs by default.
#show par: highlight

#table(
  columns: 3,
  [A],
  block[B],
  par[C],
)

--- grid-cell-in-table render ---
// Error: 8-19 cannot use `grid.cell` as a table cell
// Hint: 8-19 use `table.cell` instead
#table(grid.cell[])

--- issue-183-table-lines render ---
// Ensure no empty lines before a table that doesn't fit into the first page.
#set page(height: 50pt)

Hello
#table(
  columns: 4,
  [1], [2], [3], [4]
)

--- issue-1388-table-row-missing render ---
// Test that a table row isn't wrongly treated like a gutter row.
#set page(height: 70pt)
#table(
  rows: 16pt,
  ..range(6).map(str).flatten(),
)

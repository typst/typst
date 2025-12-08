// Test basic styling using the grid.cell element.

--- grid-cell-override paged ---
// Cell override
#grid(
  align: left,
  fill: red,
  stroke: blue,
  inset: 5pt,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  grid.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A],
  grid.cell(align: horizon)[G2], [A\ A\ A],
  grid.cell(inset: 0pt)[I], [F],
  [H], grid.cell(fill: blue)[J]
)

--- grid-cell-show paged ---
// Cell show rule
#show grid.cell: it => [Zz]

#grid(
  align: left,
  fill: red,
  stroke: blue,
  inset: 5pt,
  columns: 2,
  [AAAAA], [BBBBB],
  [A], [B],
  grid.cell(align: right)[C], [D],
  align(right)[E], [F],
  align(horizon)[G], [A\ A\ A]
)

--- grid-cell-show-and-override paged ---
#show grid.cell: it => (it.align, it.fill)
#grid(
  align: left,
  row-gutter: 5pt,
  [A],
  grid.cell(align: right)[B],
  grid.cell(fill: aqua)[B],
)

--- grid-cell-set paged ---
// Cell set rules
#set grid.cell(align: center)
#show grid.cell: it => (it.align, it.fill, it.inset)
#set grid.cell(inset: 20pt)
#grid(
  align: left,
  row-gutter: 5pt,
  [A],
  grid.cell(align: right)[B],
  grid.cell(fill: aqua)[B],
)

--- grid-cell-folding paged ---
// Test folding per-cell properties (align and inset)
#grid(
  columns: (1fr, 1fr),
  rows: (2.5em, auto),
  align: right,
  inset: 5pt,
  fill: (x, y) => (green, aqua).at(calc.rem(x + y, 2)),
  [Top], grid.cell(align: bottom)[Bot],
  grid.cell(inset: (bottom: 0pt))[Bot], grid.cell(inset: (bottom: 0pt))[Bot]
)

--- grid-cell-align-override paged ---
// Test overriding outside alignment
#set align(bottom + right)
#grid(
  columns: (1fr, 1fr),
  rows: 2em,
  align: auto,
  fill: green,
  [BR], [BR],
  grid.cell(align: left, fill: aqua)[BL], grid.cell(align: top, fill: red.lighten(50%))[TR]
)

--- grid-cell-various-overrides paged ---
#grid(
  columns: 2,
  fill: red,
  align: left,
  inset: 5pt,
  [ABC], [ABC],
  grid.cell(fill: blue)[C], [D],
  grid.cell(align: center)[E], [F],
  [G], grid.cell(inset: 0pt)[H]
)

--- grid-cell-show-emph paged ---
#{
  show grid.cell: emph
  grid(
    columns: 2,
    gutter: 3pt,
    [Hello], [World],
    [Sweet], [Italics]
  )
}

--- grid-cell-show-based-on-position paged ---
// Style based on position
#{
  show grid.cell: it => {
    if it.y == 0 {
      strong(it)
    } else if it.x == 1 {
      emph(it)
    } else {
      it
    }
  }
  grid(
    columns: 3,
    gutter: 3pt,
    [Name], [Age], [Info],
    [John], [52], [Nice],
    [Mary], [50], [Cool],
    [Jake], [49], [Epic]
  )
}

--- table-cell-in-grid paged ---
// Error: 7-19 cannot use `table.cell` as a grid cell
// Hint: 7-19 use `grid.cell` instead
#grid(table.cell[])

--- issue-5723-grid-heading-numbering paged ---
#set heading(numbering: "1.1.")
#set page(width: 150pt, height: 3.5cm)

#table(
  columns: (1fr, 2fr),
  [= A],
  [= B],
  [
    = C
    #lines(4)
    = D
  ],
  table(
    columns: (1fr, 1fr),
    ..([
      = X
      #lines(2)
      = Y
      #lines(2)
    ],) * 2
  ),
  [= E],
  [= F]
)

--- issue-7188-grid-counter-order paged ---
#set page(height: 1cm)

#let word-numbering(body) = {
  let num = counter("_linenumbered")
  let word-label = <_word>
  show word-label: _ => {
    num.step()
    box(width: 0pt, super(numbering("1", num.get().first())))
  }
  show regex("\w+\.?"): it => it + [#metadata(none)#word-label]
  body
}

#grid(
  columns: 1,
  word-numbering(lorem(8))
)

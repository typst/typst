--- line-numbers-enable paged ---
#set page(margin: (left: 2.5em))
#set par.line(numbering: "1")

First line \
Second line \
Third line

--- line-numbers-clearance paged ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1", number-clearance: 0cm)

First line \
Second line \
Third line

--- line-numbers-margin paged ---
#set page(margin: (right: 3cm))
#set par.line(numbering: "1", number-clearance: 1.5cm, number-margin: end)

First line \
Second line \
Third line

--- line-numbers-default-alignment paged ---
#set page(margin: (left: 3em))
#set par.line(numbering: "1")
a
#([\ a] * 15)

--- line-numbers-start-alignment paged ---
#set page(margin: (left: 3em))
#set par.line(numbering: "i", number-align: start)
a \
a
#pagebreak()
a \
a \
a

--- line-numbers-auto-alignment paged ---
#set page(margin: (right: 3cm))
#set par.line(numbering: "i", number-clearance: 1.5cm, number-margin: end)

First line \
Second line \
Third line

--- line-numbers-rtl paged ---
#set page(margin: (right: 3em))
#set text(dir: rtl)
#set par.line(numbering: "1")
a
#([\ a] * 15)

--- line-numbers-columns paged ---
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "1", number-clearance: 0.5em)

Hello \
Beautiful \
World
#colbreak()
Birds \
In the \
Sky

--- line-numbers-columns-alignment paged ---
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "i", number-clearance: 0.5em)

Hello \
Beautiful \
World
#colbreak()
Birds \
In the \
Sky

--- line-numbers-multi-columns paged ---
#set page(columns: 3, margin: (x: 1.5em))
#set par.line(numbering: "1", number-clearance: 0.5em)

A \
B \
C
#colbreak()
D \
E \
F
#colbreak()
G \
H \
I

--- line-numbers-columns-rtl paged ---
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "1", number-clearance: 0.5em)
#set text(dir: rtl)

Hello \
Beautiful \
World
#colbreak()
Birds \
In the \
Sky

--- line-numbers-columns-override paged ---
#set columns(gutter: 1.5em)
#set page(columns: 2, margin: (x: 1.5em))
#set par.line(numbering: "1", number-margin: end, number-clearance: 0.5em)

Hello \
Beautiful \
World
#colbreak()
Birds \
In the \
Sky

--- line-numbers-page-scope paged ---
#set page(margin: (left: 2.5em))
#set par.line(numbering: "1", numbering-scope: "page")

First line \
Second line
#pagebreak()
Back to first line \
Second line again
#page[
  Once again, first \
  And second
]
Back to first

--- line-numbers-page-scope-with-columns paged ---
#set page(margin: (x: 1.1cm), columns: 2)
#set par.line(
  numbering: "1",
  number-clearance: 0.5cm,
  numbering-scope: "page"
)

A \
A \
A
#colbreak()
B \
B \
B
#pagebreak()
One \
Two \
Three
#colbreak()
Four \
Five \
Six
#page[
  Page \
  Elem
  #colbreak()
  Number \
  Reset
]
We're back
#colbreak()
Bye!

--- line-numbers-page-scope-quasi-empty-first-column paged ---
// Ensure this case (handled separately internally) is properly handled.
#set page(margin: (x: 1.1cm), height: 2cm, columns: 2)
#set par.line(
  numbering: "1",
  number-clearance: 0.5cm,
  numbering-scope: "page"
)

First line
#colbreak()
Second line
#pagebreak()
#place[]
#box(height: 2cm)[First!]

--- line-numbers-nested-content paged ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1", number-clearance: 0.5cm)

#grid(
  columns: (1fr, 1fr),
  column-gutter: 0.5cm,
  inset: 5pt,
  block[A\ #box(lorem(5))], [Roses\ are\ red],
  [AAA], [],
  [], block[BBB\ CCC],
)

--- line-numbers-place-out-of-order paged ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1", number-clearance: 0.5cm)

#place(bottom)[Line 4]

Line 1\
Line 2\
Line 3
#v(1cm)

--- line-numbers-deduplication paged ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1", number-clearance: 0.5cm)

#grid(
  columns: (1fr, 1fr),
  column-gutter: 0.5cm,
  row-gutter: 5pt,
  lorem(5), [A\ B\ C],
  [DDD], [DDD],
  [This is], move(dy: 2pt)[tough]
)

--- line-numbers-deduplication-tall-line paged ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: "1", number-clearance: 0.5cm)

#grid(
  columns: (1fr, 1fr),
  column-gutter: 0.5cm,
  stroke: 0.5pt,

  grid.cell(rowspan: 2)[very #box(fill: red, height: 4cm)[tall]],
  grid.cell(inset: (y: 0.5pt))[Line 1\ Line 2\ Line 3],
  grid.cell(inset: (y: 0.5pt))[Line 4\ Line 5\ Line 6\ Line 7\ Line 8\ Line 9\ End]
)

--- line-numbers-deduplication-zero-height-number paged ---
#set page(margin: (left: 1.5cm))
#set par.line(numbering: n => move(dy: -0.6em, box(height: 0pt)[#n]), number-clearance: 0.5cm)

#grid(
  columns: (1fr, 1fr),
  column-gutter: 0.5cm,
  row-gutter: 5pt,
  lorem(5), [A\ B\ C],
  [DDD], [DDD],
  [This is], move(dy: 3pt)[tough]
)

--- line-numbers-equation-number paged ---
#set page(margin: (left: 2.5em))
#set par.line(numbering: "1")
#set math.equation(numbering: "(1)")

A
$ x $
B

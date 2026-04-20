// Defines a function for showing a table in the documentation. Ideally, we
// could just use the native table function, but the styling we want is
// currently hard to achieve with it and we also want to infer the number of
// columns from the header.

#import "system.typ": colors

// Displays a table in the documentation.
//
// Infers the number of columns from the `header` (must be the first child).
#let docs-table(header, ..args) = {
  assert.eq(
    header.func(),
    table.header,
    message: "first argument must be a table header",
  )
  table(
    columns: header.children.len(),
    column-gutter: 1fr,
    row-gutter: (0.25em, 0pt),
    table.hline(stroke: 0.75pt + colors.light-gray.shade-50, position: bottom),
    table.header(..header.children.map(it => {
      table.cell(inset: (bottom: 0.75em, rest: 4pt), strong(it.body))
    })),
    stroke: none,
    ..args,
  )
}

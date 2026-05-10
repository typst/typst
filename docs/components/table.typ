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

  let columns = header.children.len()
  let base = table.with(
    columns: columns,
    row-gutter: (0.25em, 0pt),
    column-gutter: 2em,
    table.hline(stroke: 0.75pt + colors.light-gray.shade-50, position: bottom),
    table.header(..header.children.map(it => {
      table.cell(inset: (bottom: 0.75em, rest: 4pt), strong(it.body))
    })),
    stroke: none,
    ..args,
  )

  context if target() == "paged" {
    layout(size => {
      if measure(base()).width < size.width {
        // If a table with all auto columns and wide gutters fits, then use the
        // wide spacing and expand the last column.
        base(columns: (auto,) * (columns - 1) + (1fr,))
      } else {
        // If such a table doesn't fit, shrink gutters and use auto columns.
        base(column-gutter: 0.5em)
      }
    })
  } else {
    base()
  }
}

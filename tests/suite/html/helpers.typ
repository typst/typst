// SKIP

// TODO: Remove this once `grid` is supported in HTML export.
#let grid(..args) = context {
  if target() != "html" {
    return std.grid(..args)
  }

  let grid-style = ("display: grid",)

  let columns = args.at("columns")
  let columns = if type(columns) == int {
    "repeat(" + str(columns) + ", min-content)"
  } else {
    columns.map(repr).join(" ")
  }
  grid-style.push("grid-template-columns: " + columns)

  let gutter = args.at("gutter", default: none)
  if gutter != none {
    grid-style.push(" gap: " + repr(gutter))
  }

  html.div(style: grid-style.join("; "), args.pos().join())
}

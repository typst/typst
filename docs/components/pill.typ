#import "system.typ": colors
#import "base.typ": classnames
#import "linking.typ": def-dest
#import "reflect.typ": ty-category-map, ty-name-map

// The different pill fills for the type categories defined in `ty.typ`.
#let pill-fill = (
  con: colors.teal.shade-20,
  str: colors.green.shade-10,
  kw: colors.red.shade-10,
  num: colors.orange.shade-10,
  obj: colors.light-gray.shade-05,
  collect: colors.purple.shade-10,
  fn: rgb("d1d4fd"),
  meta: rgb("c6d6ec"),
  layout: colors.blue.shade-10,
  date: gradient.linear(
    (rgb("ebf4f9"), 0%),
    (rgb("eff0ec"), 25%),
    (rgb("f4ebdd"), 45%),
    (rgb("f4e8d9"), 66%),
    (rgb("f5e7d8"), 75%),
    (rgb("dbd1ce"), 100%),
  ),
  tiling: tiling(size: (10pt, 10pt), {
    place(rect(
      fill: rgb("#ffd2ec"),
      width: 10pt,
      height: 10pt,
    ))
    for x in range(-1, 2) {
      place(dx: x * 10pt, line(
        start: (-10pt, -10pt),
        end: (20pt, 20pt),
        stroke: 4pt + rgb("c6feff"),
      ))
    }
  }),
  col: gradient.linear(
    (rgb(124, 213, 255), 0%),
    (rgb(166, 251, 202), 33%),
    (rgb(255, 243, 124), 66%),
    (rgb(255, 164, 157), 100%),
    angle: -7deg,
  ),
)

// Renders a "pill" (a small colorful box) for a type. This is used for
// parameter definitions in the docs.
#let ty-pill(ty, linked: true) = context {
  let r = repr(ty)
  let c = ty-category-map.at(r, default: "obj")
  let name = ty-name-map.at(r, default: r)
  let linked = linked and ty != "any"

  if target() == "paged" {
    let inner = raw(name)
    let body = if linked { link(def-dest(ty), inner) } else { inner }
    box(
      fill: pill-fill.at(c),
      inset: (x: 0.3em),
      outset: (y: 0.3em),
      radius: 0.3em,
      // Make the pill contents a little smaller because the pill itself also
      // adds visual weight.
      text(0.9em, body),
    )
  } else {
    let class = classnames("pill", "pill-" + c)
    if linked {
      // This is the only way to attach a class to the native link.
      set html.elem(attrs: (class: class))
      link(def-dest(ty), name)
    } else {
      html.span(class: class, name)
    }
  }
}

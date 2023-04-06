// Test shape fill & stroke.

---
#let variant = rect.with(width: 20pt, height: 10pt)
#let items = for (i, item) in (
  variant(stroke: none),
  variant(),
  variant(fill: none),
  variant(stroke: 2pt),
  variant(stroke: eastern),
  variant(stroke: eastern + 2pt),
  variant(fill: eastern),
  variant(fill: eastern, stroke: none),
  variant(fill: forest, stroke: none),
  variant(fill: forest, stroke: conifer),
  variant(fill: forest, stroke: black + 2pt),
  variant(fill: forest, stroke: conifer + 2pt),
).enumerate() {
  (align(horizon)[#(i + 1).], item, [])
}

#grid(
  columns: (auto, auto, 1fr, auto, auto, 0fr),
  gutter: 5pt,
  ..items,
)

---
// Test stroke folding.
#let sq(..args) = box(square(size: 10pt, ..args))

#set square(stroke: none)
#sq()
#set square(stroke: auto)
#sq()
#sq(fill: teal)
#sq(stroke: 2pt)
#sq(stroke: blue)
#sq(fill: teal, stroke: blue)
#sq(fill: teal, stroke: 2pt + blue)

---
// Test stroke composition.
#set square(stroke: 4pt)
#set text(font: "Roboto")
#square(
  stroke: (left: red, top: yellow, right: green, bottom: blue),
  radius: 100%, align(center+horizon)[*G*],
  inset: 8pt
)

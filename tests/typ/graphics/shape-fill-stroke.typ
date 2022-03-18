// Test shape fill & stroke.

---
#let variant = rect.with(width: 20pt, height: 10pt)
#let items = for i, item in (
  variant(stroke: none),
  variant(),
  variant(fill: none),
  variant(thickness: 2pt),
  variant(stroke: eastern),
  variant(stroke: eastern, thickness: 2pt),
  variant(fill: eastern),
  variant(fill: eastern, stroke: none),
  variant(fill: forest, stroke: none, thickness: 2pt),
  variant(fill: forest, stroke: conifer),
  variant(fill: forest, stroke: black, thickness: 2pt),
  variant(fill: forest, stroke: conifer, thickness: 2pt),
) {
  (align(horizon)[{i + 1}.], item, [])
}

#grid(
  columns: (auto, auto, 1fr, auto, auto, 0fr),
  gutter: 5pt,
  ..items,
)

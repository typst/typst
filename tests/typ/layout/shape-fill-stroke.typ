// Test shape fill & stroke.

---
#let rect with (width: 20pt, height: 10pt)
#let items = for i, rect in (
  rect(stroke: none),
  rect(),
  rect(fill: none),
  rect(thickness: 2pt),
  rect(stroke: eastern),
  rect(stroke: eastern, thickness: 2pt),
  rect(fill: eastern),
  rect(fill: eastern, stroke: none),
  rect(fill: forest, stroke: none, thickness: 2pt),
  rect(fill: forest, stroke: conifer),
  rect(fill: forest, stroke: black, thickness: 2pt),
  rect(fill: forest, stroke: conifer, thickness: 2pt),
) {
  (align(horizon)[{i + 1}.], rect, [])
}

#grid(
  columns: (auto, auto, 1fr, auto, auto, 0fr),
  gutter: 5pt,
  ..items,
)

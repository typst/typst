// Test curves.

--- curve ---
#set page(height: 300pt, width: 200pt)
#table(
  columns: (1fr, 1fr),
  rows: (1fr, 1fr, 1fr),
  align: center + horizon,
  curve(
    fill: red,
    curve.move((0%, 0%)),
    curve.cubic((-4%, 4%), (54%, 46%), (50%, 50%)),
    curve.cubic(auto, (4%, 54%), (0%, 50%)),
    curve.cubic(auto, (54%, 4%), (50%, 0%)),
    curve.close(),
  ),
  curve(
    fill: purple,
    stroke: 1pt,
    curve.move((0pt, 0pt)),
    curve.line((30pt, 30pt)),
    curve.line((0pt, 30pt)),
    curve.line((30pt, 0pt)),
  ),
  curve(
    fill: blue,
    stroke: 1pt,
    curve.move((30%, 0%)),
    curve.cubic((10%, 0%), (10%, 60%), (30%, 60%)),
    curve.cubic(none, (110%, 0%), (50%, 30%)),
    curve.cubic((110%, 30%), (65%, 30%), (30%, 0%)),
    curve.close(mode: "line")
  ),
  curve(
    stroke: 5pt,
    curve.move((0pt,  30pt)),
    curve.line((30pt, 30pt)),
    curve.line((15pt, 0pt)),
    curve.close()
  ),
  curve(
    fill: red,
    fill-rule: "non-zero",
    curve.move((25pt, 0pt)),
    curve.line((10pt, 50pt)),
    curve.line((50pt, 20pt)),
    curve.line((0pt, 20pt)),
    curve.line((40pt, 50pt)),
    curve.close()
  ),
  curve(
    fill: red,
    fill-rule: "even-odd",
    curve.move((25pt, 0pt)),
    curve.line((10pt, 50pt)),
    curve.line((50pt, 20pt)),
    curve.line((0pt, 20pt)),
    curve.line((40pt, 50pt)),
    curve.close()
  ),
  curve(
    fill: yellow,
    fill-rule: "even-odd",
    stroke: black,
    curve.move((10pt, 10pt)),
    curve.line((20pt, 10pt)),
    curve.line((20pt, 20pt)),
    curve.close(),
    curve.move((0pt, 5pt)),
    curve.line((25pt, 5pt)),
    curve.line((25pt, 30pt)),
    curve.close(),
  ),
  curve(
    fill: yellow,
    stroke: black,
    curve.move((10pt, 10pt)),
    curve.cubic((5pt, 20pt), (15pt, 20pt), (20pt, 0pt), relative: true),
    curve.cubic(auto, (15pt, -10pt), (20pt, 0pt), relative: true),
    curve.close(mode: "line")
  ),
  curve(
    curve.move((0pt, 0pt)),
    curve.quad((10pt, 0pt), (10pt, 10pt)),
    curve.quad(auto, (10pt, 10pt), relative: true),
    curve.close(mode: "line")
  ),
  curve(
    curve.quad((10pt, 0pt), (10pt, 10pt)),
    curve.quad(auto, (10pt, 10pt), relative: true),
    curve.close(mode: "line")
  ),
)

--- curve-infinite-length ---
// Error: 2-67 cannot create curve with infinite length
#curve(curve.move((0pt, 0pt)), curve.line((float.inf * 1pt, 0pt)))

--- issue-curve-in-sized-container ---
// Curves/Paths used to implement `LayoutMultiple` rather than `LayoutSingle`
// without fulfilling the necessary contract of respecting region expansion.
#block(
  fill: aqua,
  width: 20pt,
  height: 15pt,
  curve(
    curve.move((0pt, 0pt)),
    curve.line((10pt, 10pt)),
  ),
)

// Test curves.

--- curve ---
#set page(height: 300pt, width: 200pt)
#table(
  columns: (1fr, 1fr),
  rows: (1fr, 1fr, 1fr),
  align: center + horizon,
  curve(
    fill: red,
    closed: true,
    curve.vertex(point: (0%, 0%), control-into: (4%, -4%), control-from: auto),
    curve.vertex(point: (50%, 50%), control-into: (4%, -4%), control-from: auto),
    curve.vertex(point: (0%, 50%), control-into: (4%, 4%), control-from: auto),
    curve.vertex(point: (50%, 0%), control-into: (4%, 4%), control-from: auto),
  ),
  curve(
    fill: purple,
    stroke: 1pt,
    curve.vertex(point: (0pt, 0pt)),
    curve.vertex(point: (30pt, 30pt)),
    curve.vertex(point: (0pt, 30pt)),
    curve.vertex(point: (30pt, 0pt)),
  ),
  curve(
    fill: blue,
    stroke: 1pt,
    curve.vertex(point: (30%, 0%), control-into: (35%, 30%), control-from: (-20%, 0%)),
    curve.vertex(point: (30%, 60%), control-into: (-20%, 0%), control-from: (0%, 0%)),
    curve.vertex(point: (50%, 30%), control-into: (60%, -30%), control-from: (60%, 0%)),
    curve.close()
  ),
  curve(
    stroke: 5pt,
    curve.vertex(point: (0pt,  30pt)),
    curve.vertex(point: (30pt, 30pt)),
    curve.vertex(point: (15pt, 0pt)),
    curve.close()
  ),
  curve(
    fill: red,
    fill-rule: "non-zero",
    closed: true,
    curve.vertex(point: (25pt, 0pt)),
    curve.vertex(point: (10pt, 50pt)),
    curve.vertex(point: (50pt, 20pt)),
    curve.vertex(point: (0pt, 20pt)),
    curve.vertex(point: (40pt, 50pt)),
  ),
  curve(
    fill: red,
    fill-rule: "even-odd",
    closed: true,
    curve.vertex(point: (25pt, 0pt)),
    curve.vertex(point: (10pt, 50pt)),
    curve.vertex(point: (50pt, 20pt)),
    curve.vertex(point: (0pt, 20pt)),
    curve.vertex(point: (40pt, 50pt)),
  ),
  curve(
    fill: yellow,
    fill-rule: "even-odd",
    stroke: black,
    curve.move(start: (10pt, 10pt)),
    curve.line(end: (20pt, 10pt)),
    curve.line(end: (20pt, 20pt)),
    curve.close(),
    curve.move(start: (0pt, 5pt)),
    curve.line(end: (25pt, 5pt)),
    curve.line(end: (25pt, 30pt)),
    curve.close(),
  ),
  curve(
    fill: yellow,
    stroke: black,
    curve.move(start: (10pt, 10pt)),
    curve.cubic(control-start:(5pt, 20pt), control-end:(15pt, 20pt), end:(20pt, 0pt), relative: true),
    curve.cubic(control-start: auto, control-end:(15pt, -10pt), end:(20pt, 0pt), relative: true),
    curve.close(mode: "line")
  ),
  curve(
    close-mode: "line",
    curve.move(start: (0pt, 0pt)),
    curve.quadratic(control: (10pt, 0pt), end:(10pt, 10pt)),
    curve.quadratic(control: auto, end:(10pt, 10pt), relative: true),
    curve.close()
  ),
  curve(
    closed: true,
    close-mode: "line",
    curve.quadratic(control: (10pt, 0pt), end:(10pt, 10pt)),
    curve.quadratic(control: auto, end:(10pt, 10pt), relative: true),
  ),
)

--- curve-infinite-length ---
// Error: 2-85 cannot create path with infinite length
#curve(curve.vertex(point: (0pt, 0pt)), curve.vertex(point: (float.inf * 1pt, 0pt)))

--- issue-curve-in-sized-container ---
// Paths used to implement `LayoutMultiple` rather than `LayoutSingle` without
// fulfilling the necessary contract of respecting region expansion.
#block(
  fill: aqua,
  width: 20pt,
  height: 15pt,
  curve(
    curve.vertex(point: (0pt, 0pt)),
    curve.vertex(point: (10pt, 10pt)),
  ),
)

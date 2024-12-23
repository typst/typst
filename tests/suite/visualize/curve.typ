// Test curves.

--- curve-move-single ---
#curve(
  stroke: 5pt,
  curve.move((0pt,  30pt)),
  curve.line((30pt, 30pt)),
  curve.line((15pt, 0pt)),
  curve.close()
)

--- curve-move-multiple-even-odd ---
#curve(
  fill: yellow,
  stroke: yellow.darken(20%),
  fill-rule: "even-odd",
  curve.move((10pt, 10pt)),
  curve.line((20pt, 10pt)),
  curve.line((20pt, 20pt)),
  curve.close(),
  curve.move((0pt, 5pt)),
  curve.line((25pt, 5pt)),
  curve.line((25pt, 30pt)),
  curve.close(mode: "smooth"),
)

--- curve-move-multiple-non-zero ---
#curve(
  fill: yellow,
  stroke: yellow.darken(20%),
  curve.move((10pt, 10pt)),
  curve.line((20pt, 10pt)),
  curve.line((20pt, 20pt)),
  curve.close(),
  curve.move((0pt, 5pt)),
  curve.line((25pt, 5pt)),
  curve.line((25pt, 30pt)),
  curve.close(mode: "smooth"),
)

--- curve-line ---
#curve(
  fill: purple,
  stroke: 3pt + purple.lighten(50%),
  curve.move((0pt, 0pt)),
  curve.line((30pt, 30pt)),
  curve.line((0pt, 30pt)),
  curve.line((30pt, 0pt)),
)

--- curve-quad-mirror ---
#curve(
  stroke: 2pt,
  curve.quad((20pt, 40pt), (40pt, 40pt), relative: true),
  curve.quad(auto, (40pt, -40pt), relative: true),
)

--- curve-cubic-mirror ---
#set page(height: 100pt)
#curve(
  fill: red,
  curve.move((0%, 0%)),
  curve.cubic((-4%, 4%), (54%, 46%), (50%, 50%)),
  curve.cubic(auto, (4%, 54%), (0%, 50%)),
  curve.cubic(auto, (54%, 4%), (50%, 0%)),
  curve.close(),
)

--- curve-cubic-inflection ---
#set page(height: 120pt)
#curve(
  fill: blue.lighten(80%),
  stroke: blue,
  curve.move((30%, 0%)),
  curve.cubic((10%, 0%), (10%, 60%), (30%, 60%)),
  curve.cubic(none, (110%, 0%), (50%, 30%)),
  curve.cubic((110%, 30%), (65%, 30%), (30%, 0%)),
  curve.close(mode: "straight")
)

--- curve-close-smooth ---
#curve(
  fill: blue.lighten(80%),
  stroke: blue,
  curve.move((0pt, 40pt)),
  curve.cubic((0pt, 70pt), (10pt, 80pt), (40pt, 80pt)),
  curve.cubic(auto, (80pt, 70pt), (80pt, 40pt)),
  curve.cubic(auto, (70pt, 0pt), (40pt, 0pt)),
  curve.close(mode: "smooth")
)

--- curve-close-straight ---
#curve(
  fill: blue.lighten(80%),
  stroke: blue,
  curve.move((0pt, 40pt)),
  curve.cubic((0pt, 70pt), (10pt, 80pt), (40pt, 80pt)),
  curve.cubic(auto, (80pt, 70pt), (80pt, 40pt)),
  curve.cubic(auto, (70pt, 0pt), (40pt, 0pt)),
  curve.close(mode: "straight")
)

--- curve-close-intersection ---
#curve(
  fill: yellow,
  stroke: black,
  curve.move((10pt, 10pt)),
  curve.cubic((5pt, 20pt), (15pt, 20pt), (20pt, 0pt), relative: true),
  curve.cubic(auto, (15pt, -10pt), (20pt, 0pt), relative: true),
  curve.close(mode: "straight")
)

--- curve-stroke-gradient ---
#set page(width: auto)
#let down = curve.line((40pt, 40pt), relative: true)
#let up = curve.line((40pt, -40pt), relative: true)

#curve(
  stroke: 4pt + gradient.linear(red, blue),
  down, up, down, up, down,
)

--- curve-fill-rule ---
#stack(
  dir: ltr,
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
  )
)

--- curve-infinite-length ---
// Error: 2-67 cannot create curve with infinite size
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

// Test that squares and circles respect their 1-1 aspect ratio.

---
// Test relative width and height and size that is smaller
// than default size.
#set page(width: 120pt, height: 70pt)
#set align(bottom)
#let centered = align.with(center + horizon)
#stack(
  dir: ltr,
  spacing: 1fr,
  square(width: 50%, centered[A]),
  square(height: 50%),
  stack(
    square(size: 10pt),
    square(size: 20pt, centered[B])
  ),
)

---
// Test alignment in automatically sized square and circle.
#set text(8pt)
#box(square(inset: 4pt)[
  Hey there, #align(center + bottom, rotate(180deg, [you!]))
])
#box(circle(align(center + horizon, [Hey.])))

---
// Test that minimum wins if both width and height are given.
#stack(
  dir: ltr,
  spacing: 2pt,
  square(width: 20pt, height: 40pt),
  circle(width: 20%, height: 100pt),
)

---
// Test square that is limited by region size.
#set page(width: 20pt, height: 10pt, margin: 0pt)
#stack(dir: ltr, square(fill: forest), square(fill: conifer))

---
// Test different ways of sizing.
#set page(width: 120pt, height: 40pt)
#stack(
  dir: ltr,
  spacing: 2pt,
  circle(radius: 5pt),
  circle(width: 10%),
  circle(height: 50%),
)

---
// Test that square doesn't overflow due to its aspect ratio.
#set page(width: 40pt, height: 25pt, margin: 5pt)
#square(width: 100%)
#square(width: 100%)[Hello there]

---
// Size cannot be relative because we wouldn't know
// relative to which axis.
// Error: 15-18 expected length or auto, found ratio
#square(size: 50%)

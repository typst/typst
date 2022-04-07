// Test that squares and circles respect their 1-1 aspect ratio.

---
// Test relative width and height and size that is smaller
// than default size.
#set page(width: 120pt, height: 70pt)
#square(width: 50%, align(bottom)[A])
#square(height: 50%)
#box(stack(square(size: 10pt), 5pt, square(size: 10pt, [B])))

---
// Test alignment in automatically sized square and circle.
#set text(8pt)
#square(padding: 4pt)[
  Hey there, #align(center + bottom, rotate(180deg, [you!]))
]
#circle(align(center + horizon, [Hey.]))

---
// Test that maximum wins if both width and height are given.
#square(width: 10pt, height: 20pt)
#circle(width: 20%, height: 10pt)

---
// Test square that is limited by region size.
#set page(width: 20pt, height: 10pt, margins: 0pt)
#stack(dir: ltr, square(fill: forest), square(fill: conifer))

---
// Test different ways of sizing.
#set page(width: 120pt, height: 40pt)
#circle(radius: 5pt)
#circle(width: 10%)
#circle(height: 50%)

---
// Test square that is overflowing due to its aspect ratio.
#set page(width: 40pt, height: 20pt, margins: 5pt)
#square(width: 100%)
#square(width: 100%)[Hello]

---
// Size cannot be relative because we wouldn't know
// relative to which axis.
// Error: 15-18 expected length, found ratio
#square(size: 50%)

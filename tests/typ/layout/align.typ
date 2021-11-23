// Test alignment.

---
#page(height: 100pt)
#stack(dir: ltr,
  align(left, square(size: 15pt, fill: eastern)),
  align(center, square(size: 20pt, fill: eastern)),
  align(right, square(size: 15pt, fill: eastern)),
)
#align(center + horizon, rect(fill: eastern, height: 10pt))
#align(bottom, stack(
  align(center, rect(fill: conifer, height: 10pt)),
  rect(fill: forest, height: 10pt),
))

---
#align(center)[
  Lorem Ipsum

  Dolor
]

---
// Ref: false
#test(type(center), "alignment")
#test(type(horizon), "alignment")
#test(type(center + horizon), "2d alignment")

---
// Error: 8-22 cannot add two horizontal alignments
#align(center + right, [A])

---
// Error: 8-20 cannot add two vertical alignments
#align(top + bottom, [A])

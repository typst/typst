// Test rounded rectangles and squares.

---
#set square(size: 20pt, stroke: 4pt)

// no radius for non-rounded corners
#stack(
  dir: ltr,
  square(),
  h(10pt),
  square(radius: 0pt),
  h(10pt),
  square(radius: -10pt),
)

#stack(
  dir: ltr,
  square(),
  h(10pt),
  square(radius: 0%),
  h(10pt),
  square(radius: -10%),
)


// small values for small radius
#stack(
  dir: ltr,
  square(radius: 1pt),
  h(10pt),
  square(radius: 5%),
  h(10pt),
  square(radius: 2pt),
)

// large values for large radius or circle
#stack(
  dir: ltr,
  square(radius: 8pt),
  h(10pt),
  square(radius: 10pt),
  h(10pt),
  square(radius: 12pt),
)

#stack(
  dir: ltr,
  square(radius: 45%),
  h(10pt),
  square(radius: 50%),
  h(10pt),
  square(radius: 55%),
)

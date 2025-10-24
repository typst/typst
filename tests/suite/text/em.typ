// Test font-relative sizing.

--- text-size-em-nesting render ---
#set text(size: 5pt)
A // 5pt
#[
  #set text(size: 2em)
  B // 10pt
  #[
    #set text(size: 1.5em + 1pt)
    C // 16pt
    #text(size: 2em)[D] // 32pt
    E // 16pt
  ]
  F // 10pt
]
G // 5pt

--- text-size-em render ---
// Test using ems in arbitrary places.
#set text(size: 5pt)
#set text(size: 2em)
#set square(fill: red)

#let size = {
  let size = 0.25em + 1pt
  for _ in range(3) {
    size *= 2
  }
  size - 3pt
}

#stack(dir: ltr, spacing: 1fr, square(size: size), square(size: 25pt))

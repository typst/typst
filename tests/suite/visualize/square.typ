// Test the `square` function.

--- square ---
// Default square.
#box(square())
#box(square[hey!])

--- square-auto-sized ---
// Test auto-sized square.
#square(fill: eastern)[
  #set text(fill: white, weight: "bold")
  Typst
]

--- square-relatively-sized-child ---
// Test relative-sized child.
#square(fill: eastern)[
  #rect(width: 10pt, height: 5pt, fill: conifer)
  #rect(width: 40%, height: 5pt, stroke: conifer)
]

--- square-contents-overflow ---
// Test text overflowing height.
#set page(width: 75pt, height: 100pt)
#square(fill: conifer)[
  But, soft! what light through yonder window breaks?
]

--- square-height-limited ---
// Test that square does not overflow page.
#set page(width: 100pt, height: 75pt)
#square(fill: conifer)[
  But, soft! what light through yonder window breaks?
]

--- square-size-width-and-height ---
// Size wins over width and height.
// Error: 09-20 unexpected argument: width
#square(width: 10cm, height: 20cm, size: 1cm, fill: rgb("eb5278"))

--- square-relative-size ---
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

--- square-circle-alignment ---
// Test alignment in automatically sized square and circle.
#set text(8pt)
#box(square(inset: 4pt)[
  Hey there, #align(center + bottom, rotate(180deg, [you!]))
])
#box(circle(align(center + horizon, [Hey.])))

--- square-circle-overspecified ---
// Test that minimum wins if both width and height are given.
#stack(
  dir: ltr,
  spacing: 2pt,
  square(width: 20pt, height: 40pt),
  circle(width: 20%, height: 40pt),
)

--- square-height-limited-stack ---
// Test square that is limited by region size.
#set page(width: 20pt, height: 10pt, margin: 0pt)
#stack(dir: ltr, square(fill: forest), square(fill: conifer))

--- square-no-overflow ---
// Test that square doesn't overflow due to its aspect ratio.
#set page(width: 40pt, height: 25pt, margin: 5pt)
#square()
#square[Hello there]

--- square-overflow-forced-width ---
// Test that a width-overflowing square is laid out regardless of the
// presence of inner content.
#set page(width: 60pt, height: 100pt)
#square(width: 150%)
#square(width: 150%)[Hello there]

--- square-overflow-forced-height ---
// Test that a height-overflowing square is laid out regardless of the
// presence of inner content.
#set page(width: 120pt, height: 60pt)
#square(height: 150%)
#square(height: 150%)[Hello there]

--- square-size-relative-invalid ---
// Size cannot be relative because we wouldn't know
// relative to which axis.
// Error: 15-18 expected length or auto, found ratio
#square(size: 50%)

--- square-rect-rounded ---
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

--- square-base ---
// Test that square sets correct base for its content.
#set page(height: 80pt)
#square(width: 40%, rect(width: 60%, height: 80%))

--- square-size-beyond-default ---
// Test that setting a square's height beyond its default sizes it correctly.
#square()
#square(height: 60pt)
#square(width: 60pt)
#square(size: 60pt)

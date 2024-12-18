// Test tilings.

--- tiling-line ---
// Tests that simple tilings work.
#set page(width: auto, height: auto, margin: 0pt)
#let t = tiling(size: (10pt, 10pt), line(stroke: 4pt, start: (0%, 0%), end: (100%, 100%)))
#rect(width: 50pt, height: 50pt, fill: t)

--- tiling-lines ---
#set page(width: auto, height: auto, margin: 0pt)

#let t = tiling(size: (10pt, 10pt), {
    place(line(stroke: 4pt, start: (0%, 0%), end: (100%, 100%)))
    place(line(stroke: 4pt, start: (100%,0%), end: (200%, 100%)))
    place(line(stroke: 4pt, start: (0%,100%), end: (100%, 200%)))
    place(line(stroke: 4pt, start: (-100%,0%), end: (0%, 100%)))
    place(line(stroke: 4pt, start: (0%,-100%), end: (100%, 0%)))
})
#rect(width: 50pt, height: 50pt, fill: t)

--- tiling-relative-self ---
// Test with relative set to `"self"`
#let t(..args) = tiling(size: (30pt, 30pt), ..args)[
  #set line(stroke: green)
  #place(top + left, line(start: (0%, 0%), end: (100%, 100%), stroke: 1pt))
  #place(top + left, line(start: (0%, 100%), end: (100%, 0%), stroke: 1pt))
]

#set page(fill: t(), width: 100pt, height: 100pt)
#rect(
  width: 100%,
  height: 100%,
  fill: t(relative: "self"),
  stroke: 1pt + green,
)

--- tiling-relative-parent ---
// Test with relative set to `"parent"`
#let t(fill, ..args) = tiling(size: (30pt, 30pt), ..args)[
  #rect(width: 100%, height: 100%, fill: fill, stroke: none)
  #place(top + left, line(start: (0%, 0%), end: (100%, 100%), stroke: 1pt))
  #place(top + left, line(start: (0%, 100%), end: (100%, 0%), stroke: 1pt))
]

#set page(fill: t(white), width: 100pt, height: 100pt)

#rect(fill: t(none, relative: "parent"), width: 100%, height: 100%, stroke: 1pt)

--- tiling-small ---
// Tests small tilings for pixel accuracy.
#box(
  width: 8pt,
  height: 1pt,
  fill: tiling(size: (1pt, 1pt), square(size: 1pt, fill: black))
)
#v(-1em)
#box(
  width: 8pt,
  height: 1pt,
  fill: tiling(size: (2pt, 1pt), square(size: 1pt, fill: black))
)

--- tiling-zero-sized ---
// Error: 15-51 tile size must be non-zero
// Hint: 15-51 try setting the size manually
#line(stroke: tiling(path((0pt, 0pt), (1em, 0pt))))

--- tiling-spacing-negative ---
// Test with spacing set to `(-10pt, -10pt)`
#let t(..args) = tiling(size: (30pt, 30pt), ..args)[
  #square(width: 100%, height: 100%, stroke: 1pt, fill: blue)
]

#set page(width: 100pt, height: 100pt)

#rect(fill: t(spacing: (-10pt, -10pt)), width: 100%, height: 100%, stroke: 1pt)

--- tiling-spacing-zero ---
// Test with spacing set to `(0pt, 0pt)`
#let t(..args) = tiling(size: (30pt, 30pt), ..args)[
  #square(width: 100%, height: 100%, stroke: 1pt, fill: blue)
]

#set page(width: 100pt, height: 100pt)

#rect(fill: t(spacing: (0pt, 0pt)), width: 100%, height: 100%, stroke: 1pt)

--- tiling-spacing-positive ---
// Test with spacing set to `(10pt, 10pt)`
#let t(..args) = tiling(size: (30pt, 30pt), ..args)[
  #square(width: 100%, height: 100%, stroke: 1pt, fill: blue)
]

#set page(width: 100pt, height: 100pt)

#rect(fill: t(spacing: (10pt, 10pt,)), width: 100%, height: 100%, stroke: 1pt)

--- tiling-stroke ---
// Test tiling on strokes
#align(
  center + top,
  square(
    size: 50pt,
    fill: tiling(
      size: (5pt, 5pt),
      align(horizon + center, circle(fill: blue, radius: 2.5pt))
    ),
    stroke: 7.5pt + tiling(
      size: (5pt, 5pt),
      align(horizon + center, circle(fill: red, radius: 2.5pt))
    )
  )
)

--- tiling-stroke-relative-parent ---
// Test tiling on strokes with relative set to `"parent"`
// The tiling on the circle should align with the tiling on the square.
#align(
  center + top,
  block(
    width: 50pt,
    height: 50pt,
    fill: tiling(size: (5pt, 5pt), circle(radius: 2.5pt, fill: blue)),
    align(center + horizon, circle(
      radius: 15pt,
      stroke: 7.5pt + tiling(
        size: (5pt, 5pt), circle(radius: 2.5pt, fill: red), relative: "parent"
      ),
    ))
  )
)

--- tiling-text ---
// Test a tiling on some text. You shouldn't be able to see the text, if you can
// then that means that the transform matrices are not being applied to the text
// correctly.
#let t = tiling(
  size: (30pt, 30pt),
  relative: "parent",
  square(size: 30pt, fill: gradient.conic(..color.map.rainbow))
);

#set page(
  width: 140pt,
  height: 140pt,
  fill: t
)

#rotate(45deg, scale(x: 50%, y: 70%, rect(
  width: 100%,
  height: 100%,
  stroke: 1pt,
)[
  #lorem(10)

  #set text(fill: t)
  #lorem(10)
]))

--- tiling-pattern-compatibility ---
#set page(width: auto, height: auto, margin: 0pt)
#let t = pattern(size: (10pt, 10pt), line(stroke: 4pt, start: (0%, 0%), end: (100%, 100%)))
#rect(width: 50pt, height: 50pt, fill: t)

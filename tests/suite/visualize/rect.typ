// Test the `rect` function.

--- rect paged ---
// Default rectangle.
#rect()

--- rect-customization paged ---
#set page(width: 150pt)

// Fit to text.
#rect(fill: conifer)[Textbox]

// Empty with fixed width and height.
#block(rect(
  height: 15pt,
  fill: rgb("46b3c2"),
  stroke: 2pt + rgb("234994"),
))

// Fixed width, text height.
#rect(width: 2cm, fill: rgb("9650d6"))[Fixed and padded]

// Page width, fixed height.
#rect(height: 1cm, width: 100%, fill: rgb("734ced"))[Topleft]

// These are inline with text.
{#box(rect(width: 0.5in, height: 7pt, fill: rgb("d6cd67")))
 #box(rect(width: 0.5in, height: 7pt, fill: rgb("edd466")))
 #box(rect(width: 0.5in, height: 7pt, fill: rgb("e3be62")))}

// Rounded corners.
#stack(
  dir: ltr,
  spacing: 1fr,
  rect(width: 2cm, radius: 30%),
  rect(width: 1cm, radius: (left: 10pt, right: 5pt)),
  rect(width: 1.25cm, radius: (
    top-left: 2pt,
    top-right: 5pt,
    bottom-right: 8pt,
    bottom-left: 11pt
  )),
)

// Different strokes.
#set rect(stroke: (right: red))
#rect(width: 100%, fill: lime, stroke: (x: 5pt, y: 1pt))

--- rect-stroke paged ---
// Rectangle strokes
#rect(width: 20pt, height: 20pt, stroke: red)
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (rest: red, top: (paint: blue, dash: "dashed")))
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (thickness: 5pt, join: "round"))

--- rect-stroke-caps paged ---
// Separated segments
#rect(width: 20pt, height: 20pt, stroke: (
  left: (cap: "round", thickness: 5pt),
  right: (cap: "square", thickness: 7pt),
))
// Joined segment with different caps.
#rect(width: 20pt, height: 20pt, stroke: (
  left: (cap: "round", thickness: 5pt),
  top: (cap: "square", thickness: 7pt),
))
// No caps when there is a radius for that corner.
#rect(width: 20pt, height: 20pt, radius: (top: 3pt), stroke: (
  left: (cap: "round", thickness: 5pt),
  top: (cap: "square", thickness: 7pt),
))
--- red-stroke-bad-type paged ---
// Error: 15-21 expected length, color, gradient, tiling, dictionary, stroke, none, or auto, found array
#rect(stroke: (1, 2))

--- rect-fill-stroke paged ---
#let variant = rect.with(width: 20pt, height: 10pt)
#let items = for (i, item) in (
  variant(stroke: none),
  variant(),
  variant(fill: none),
  variant(stroke: 2pt),
  variant(stroke: eastern),
  variant(stroke: eastern + 2pt),
  variant(fill: eastern),
  variant(fill: eastern, stroke: none),
  variant(fill: forest, stroke: none),
  variant(fill: forest, stroke: conifer),
  variant(fill: forest, stroke: black + 2pt),
  variant(fill: forest, stroke: conifer + 2pt),
).enumerate() {
  (align(horizon)[#(i + 1).], item, [])
}

#grid(
  columns: (auto, auto, 1fr, auto, auto, 0fr),
  gutter: 5pt,
  ..items,
)

--- rect-radius-bad-key paged ---
// Error: 15-38 unexpected key "cake", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#rect(radius: (left: 10pt, cake: 5pt))

--- issue-1825-rect-overflow paged ---
#set page(width: 17.8cm)
#set par(justify: true)
#rect(lorem(70))

--- issue-3264-rect-negative-dimensions paged ---
// Negative dimensions
#rect(width: -1cm, fill: gradient.linear(red, blue))[Reverse left]

#rect(width: 1cm, fill: gradient.linear(red, blue))[Left]

#align(center, rect(width: -1cm, fill: gradient.linear(red, blue))[Reverse center])

#align(center, rect(width: 1cm, fill: gradient.linear(red, blue))[Center])

#align(right, rect(width: -1cm, fill: gradient.linear(red, blue))[Reverse right])

#align(right, rect(width: 1cm, fill: gradient.linear(red, blue))[Right])

--- rect-size-beyond-default paged ---
// Test that setting a rectangle's height beyond its default sizes it correctly.
#rect()
#rect(height: 60pt)
#rect(width: 60pt)

--- rect-stroke-variations-without-radius paged ---
#import "block.typ": test-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [butt], [square], [round],
  [no dash],
  test-block(cap: "butt"),
  test-block(cap: "square"),
  test-block(cap: "round"),

  [dashed],
  test-block(cap: "butt", dash: "dashed"),
  test-block(cap: "square", dash: "dashed"),
  test-block(cap: "round", dash: "dashed"),

  [loosely-dashed],
  test-block(cap: "butt", dash: "loosely-dashed"),
  test-block(cap: "square", dash: "loosely-dashed"),
  test-block(cap: "round", dash: "loosely-dashed"),
)

--- rect-stroke-variations-with-radius paged ---
#import "block.typ": test-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [butt], [square], [round],
  [no dash],
  test-block(cap: "butt", radius: 12pt),
  test-block(cap: "square", radius: 12pt),
  test-block(cap: "round", radius: 12pt),

  [dashed],
  test-block(cap: "butt", radius: 12pt, dash: "dashed"),
  test-block(cap: "square", radius: 12pt, dash: "dashed"),
  test-block(cap: "round", radius: 12pt, dash: "dashed"),

  [loosely-dashed],
  test-block(cap: "butt", radius: 12pt, dash: "loosely-dashed"),
  test-block(cap: "square", radius: 12pt, dash: "loosely-dashed"),
  test-block(cap: "round", radius: 12pt, dash: "loosely-dashed"),
)

--- rect-stroke-variations-with-radius-and-adjacent-zero-width-stroke paged ---
#import "block.typ": test-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [butt], [square], [round],
  [no dash],
  test-block(cap: "butt", radius: 12pt, adjacent: 0pt),
  test-block(cap: "square", radius: 12pt, adjacent: 0pt),
  test-block(cap: "round", radius: 12pt, adjacent: 0pt),

  [dashed],
  test-block(cap: "butt", radius: 12pt, adjacent: 0pt, dash: "dashed"),
  test-block(cap: "square", radius: 12pt, adjacent: 0pt, dash: "dashed"),
  test-block(cap: "round", radius: 12pt, adjacent: 0pt, dash: "dashed"),

  [loosely-dashed],
  test-block(cap: "butt", radius: 12pt, adjacent: 0pt, dash: "loosely-dashed"),
  test-block(cap: "square", radius: 12pt, adjacent: 0pt, dash: "loosely-dashed"),
  test-block(cap: "round", radius: 12pt, adjacent: 0pt, dash: "loosely-dashed"),
)

--- rect-stroke-variations-with-radius-and-adjacent-thin-stroke paged ---
#import "block.typ": test-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [butt], [square], [round],
  [no dash],
  test-block(cap: "butt", radius: 12pt, adjacent: 1pt),
  test-block(cap: "square", radius: 12pt, adjacent: 1pt),
  test-block(cap: "round", radius: 12pt, adjacent: 1pt),

  [dashed],
  test-block(cap: "butt", radius: 12pt, adjacent: 1pt, dash: "dashed"),
  test-block(cap: "square", radius: 12pt, adjacent: 1pt, dash: "dashed"),
  test-block(cap: "round", radius: 12pt, adjacent: 1pt, dash: "dashed"),

  [loosely-dashed],
  test-block(cap: "butt", radius: 12pt, adjacent: 1pt, dash: "loosely-dashed"),
  test-block(cap: "square", radius: 12pt, adjacent: 1pt, dash: "loosely-dashed"),
  test-block(cap: "round", radius: 12pt, adjacent: 1pt, dash: "loosely-dashed"),
)

--- rect-cap-variations-radius-4-3-of-stroke-thickness paged ---
#import "block.typ": another-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [none], [0 width], [thin],
  [butt],
  another-block(cap: "butt", radius: 8pt, adjacent: none),
  another-block(cap: "butt", radius: 8pt, adjacent: 0pt),
  another-block(cap: "butt", radius: 8pt, adjacent: 1pt),

  [square],
  another-block(cap: "square", radius: 8pt, adjacent: none),
  another-block(cap: "square", radius: 8pt, adjacent: 0pt),
  another-block(cap: "square", radius: 8pt, adjacent: 1pt),

  [round],
  another-block(cap: "round", radius: 8pt, adjacent: none),
  another-block(cap: "round", radius: 8pt, adjacent: 0pt),
  another-block(cap: "round", radius: 8pt, adjacent: 1pt),
)

--- rect-cap-variations-radius-same-as-stroke-thickness paged ---
#import "block.typ": another-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [none], [0 width], [thin],
  [butt],
  another-block(cap: "butt", adjacent: none),
  another-block(cap: "butt", adjacent: 0pt),
  another-block(cap: "butt", adjacent: 1pt),

  [square],
  another-block(cap: "square", adjacent: none),
  another-block(cap: "square", adjacent: 0pt),
  another-block(cap: "square", adjacent: 1pt),

  [round],
  another-block(cap: "round", adjacent: none),
  another-block(cap: "round", adjacent: 0pt),
  another-block(cap: "round", adjacent: 1pt),
)

--- rect-cap-variations-radius-2-3-of-stroke-thickness paged ---
#import "block.typ": another-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [none], [0 width], [thin],
  [butt],
  another-block(cap: "butt", radius: 4pt, adjacent: none),
  another-block(cap: "butt", radius: 4pt, adjacent: 0pt),
  another-block(cap: "butt", radius: 4pt, adjacent: 1pt),

  [square],
  another-block(cap: "square", radius: 4pt, adjacent: none),
  another-block(cap: "square", radius: 4pt, adjacent: 0pt),
  another-block(cap: "square", radius: 4pt, adjacent: 1pt),

  [round],
  another-block(cap: "round", radius: 4pt, adjacent: none),
  another-block(cap: "round", radius: 4pt, adjacent: 0pt),
  another-block(cap: "round", radius: 4pt, adjacent: 1pt),
)

--- rect-cap-variations-radius-1-3-of-stroke-thickness paged ---
#import "block.typ": another-block
#set page(width: 7.5cm, margin: 0pt)
#table(
  stroke: none,
  columns: (0.75fr,) + 3 * (1fr,),
  [], [none], [0 width], [thin],
  [butt],
  another-block(cap: "butt", radius: 2pt, adjacent: none),
  another-block(cap: "butt", radius: 2pt, adjacent: 0pt),
  another-block(cap: "butt", radius: 2pt, adjacent: 1pt),

  [square],
  another-block(cap: "square", radius: 2pt, adjacent: none),
  another-block(cap: "square", radius: 2pt, adjacent: 0pt),
  another-block(cap: "square", radius: 2pt, adjacent: 1pt),

  [round],
  another-block(cap: "round", radius: 2pt, adjacent: none),
  another-block(cap: "round", radius: 2pt, adjacent: 0pt),
  another-block(cap: "round", radius: 2pt, adjacent: 1pt),
)

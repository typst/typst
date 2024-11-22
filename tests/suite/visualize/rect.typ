// Test the `rect` function.

--- rect ---
// Default rectangle.
#rect()

--- rect-customization ---
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

--- rect-stroke ---
// Rectangle strokes
#rect(width: 20pt, height: 20pt, stroke: red)
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (rest: red, top: (paint: blue, dash: "dashed")))
#v(3pt)
#rect(width: 20pt, height: 20pt, stroke: (thickness: 5pt, join: "round"))

--- red-stroke-bad-type ---
// Error: 15-21 expected length, color, gradient, pattern, dictionary, stroke, none, or auto, found array
#rect(stroke: (1, 2))

--- rect-fill-stroke ---
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

--- rect-radius-bad-key ---
// Error: 15-38 unexpected key "cake", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#rect(radius: (left: 10pt, cake: 5pt))

--- issue-1825-rect-overflow ---
#set page(width: 17.8cm)
#set par(justify: true)
#rect(lorem(70))

--- issue-3264-rect-negative-dimensions ---
// Negative dimensions
#rect(width: -1cm, fill: gradient.linear(red, blue))[Reverse left]

#rect(width: 1cm, fill: gradient.linear(red, blue))[Left]

#align(center, rect(width: -1cm, fill: gradient.linear(red, blue))[Reverse center])

#align(center, rect(width: 1cm, fill: gradient.linear(red, blue))[Center])

#align(right, rect(width: -1cm, fill: gradient.linear(red, blue))[Reverse right])

#align(right, rect(width: 1cm, fill: gradient.linear(red, blue))[Right])

--- rectangle-sizing ---
#set page(width: 6cm)
// Test that we can set one of the rectangle's dimensions beyond its default.
#rect(height: 5cm)
#rect(width: 5cm)

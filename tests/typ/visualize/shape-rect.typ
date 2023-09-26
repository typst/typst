// Test the `rect` function.

---
// Default rectangle.
#rect()

---
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

---
// Error: 15-38 unexpected key "cake", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#rect(radius: (left: 10pt, cake: 5pt))

---
// Error: 15-21 expected length, color, dictionary, stroke, none, or auto, found array
#rect(stroke: (1, 2))

---
#set page(width: 17.8cm)
#lorem(100)
#rect(lorem(100))
#set par(justify: true)
#lorem(100)
#rect(lorem(100))

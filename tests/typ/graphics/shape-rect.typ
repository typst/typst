// Test the `rect` function.

---
// Default rectangle.
#rect()

---
#set page(width: 150pt)

// Fit to text.
#rect(fill: conifer, inset: 3pt)[Textbox]

// Empty with fixed width and height.
#block(rect(
  height: 15pt,
  fill: rgb("46b3c2"),
  stroke: 2pt + rgb("234994"),
))

// Fixed width, text height.
#rect(width: 2cm, fill: rgb("9650d6"), inset: 5pt)[Fixed and padded]

// Page width, fixed height.
#rect(height: 1cm, width: 100%, fill: rgb("734ced"))[Topleft]

// These are inline with text.
\{#rect(width: 0.5in, height: 7pt, fill: rgb("d6cd67"))
  #rect(width: 0.5in, height: 7pt, fill: rgb("edd466"))
  #rect(width: 0.5in, height: 7pt, fill: rgb("e3be62"))\}

// Rounded corners.
#rect(width: 2cm, radius: 60%)
#rect(width: 1cm, radius: (left: 10pt, right: 5pt))
#rect(width: 1.25cm, radius: (
  top-left: 2pt,
  top-right: 5pt,
  bottom-right: 8pt,
  bottom-left: 11pt
))

// Different strokes.
[
  #set rect(stroke: (right: red))
  #rect(width: 100%, fill: lime, stroke: (x: 5pt, y: 1pt))
]

---
// Outset padding.
#set raw(lang: "rust")
#show raw: it => [
  #set text(8pt)
  #h(5.6pt, weak: true)
  #rect(radius: 3pt, outset: (y: 3pt, x: 2.5pt), fill: rgb(239, 241, 243), it)
  #h(5.6pt, weak: true)
]

Use the `*const T` pointer or the `&mut T` reference.

---
// Error: 15-38 unexpected key "cake"
#rect(radius: (left: 10pt, cake: 5pt))

---
// Error: 15-21 expected stroke or none or dictionary with any of `left`, `top`, `right`, `bottom`, `x`, `y`, or `rest` as keys or auto, found array
#rect(stroke: (1, 2))

---
// Error: 15-19 expected relative length or none or dictionary with any of `top-left`, `top-right`, `bottom-right`, `bottom-left`, `left`, `top`, `right`, `bottom`, or `rest` as keys, found color
#rect(radius: blue)

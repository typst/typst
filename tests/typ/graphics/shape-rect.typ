// Test the `rect` function.

---
// Default rectangle.
#rect()

---
#set page(width: 150pt)

// Fit to text.
#rect(fill: conifer, padding: 3pt)[Textbox]

// Empty with fixed width and height.
#block(rect(
  height: 15pt,
  fill: rgb("46b3c2"),
  stroke: 2pt + rgb("234994"),
))

// Fixed width, text height.
#rect(width: 2cm, fill: rgb("9650d6"), padding: 5pt)[Fixed and padded]

// Page width, fixed height.
#rect(height: 1cm, width: 100%, fill: rgb("734ced"))[Topleft]

// These are inline with text.
\{#rect(width: 0.5in, height: 7pt, fill: rgb("d6cd67"))
  #rect(width: 0.5in, height: 7pt, fill: rgb("edd466"))
  #rect(width: 0.5in, height: 7pt, fill: rgb("e3be62"))\}

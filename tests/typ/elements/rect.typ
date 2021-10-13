// Test the `rect` function.

---
// Default rectangle.
#rect()

---
#page(width: 150pt)

// Fit to text.
#rect(fill: conifer)[Textbox]

// Empty with fixed width and height.
#rect(width: 3cm, height: 12pt, fill: rgb("ed8a4c"))

// Fixed width, text height.
#rect(width: 2cm, fill: rgb("9650d6"), pad(5pt)[Fixed and padded])

// Page width, fixed height.
#rect(height: 1cm, width: 100%, fill: rgb("734ced"))[Topleft]

// These are inline with text.
\{#rect(width: 0.5in, height: 7pt, fill: rgb("d6cd67"))
  #rect(width: 0.5in, height: 7pt, fill: rgb("edd466"))
  #rect(width: 0.5in, height: 7pt, fill: rgb("e3be62"))\}

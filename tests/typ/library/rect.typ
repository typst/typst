// Test shapes.

---
// Test the `rect` function.

#page(width: 150pt)

// Fit to text.
#rect(fill: #9feb52)[Textbox]

// Empty with fixed width and height.
#rect(width: 3cm, height: 12pt, fill: #CB4CED)

// Fixed width, text height.
#rect(width: 2cm, fill: #9650D6, pad(5pt)[Fixed and padded])

// Page width, fixed height.
#rect(height: 1cm, width: 100%, fill: #734CED)[Topleft]

// Not visible, but creates a gap between the boxes above and below
// due to line spacing.
#rect(width: 1in, fill: #ff0000)

// These are in a row!
#rect(width: 0.5in, height: 10pt, fill: #D6CD67)
#rect(width: 0.5in, height: 10pt, fill: #EDD466)
#rect(width: 0.5in, height: 10pt, fill: #E3BE62)

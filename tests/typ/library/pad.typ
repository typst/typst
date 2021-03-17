// Test the `pad` function.

---
// Use for indentation.
#pad(left: 10pt, [Indented!])

// All sides together.
#rect(fill: #9feb52,
  pad(10pt, right: 20pt,
  rect(width: 20pt, height: 20pt, fill: #eb5278)))

// Error: 14-24 missing argument: body
Hi #rect(pad(left: 10pt)) there

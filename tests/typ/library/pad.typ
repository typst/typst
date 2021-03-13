// Test the `pad` function.

---
// Use for indentation.
#pad(left: 10pt, [Indented!])

// All sides together.
#box(color: #9feb52,
  pad(10pt, right: 20pt,
  box(color: #eb5278, width: 20pt, height: 20pt)))

// Error: 13-23 missing argument: body
Hi #box(pad(left: 10pt)) there

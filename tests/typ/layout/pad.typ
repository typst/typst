// Test the `pad` function.

---
// Use for indentation.
#pad(left: 10pt, [Indented!])

// All sides together.
#rect(fill: conifer)[
  #pad!(10pt, right: 20pt)
  #rect(width: 20pt, height: 20pt, fill: #eb5278)
]

// Error: 13-23 missing argument: body
Hi #box(pad(left: 10pt)) there

---
#let pad(body) = pad(left: 10pt, right: 10pt, body)

// Pad inherits expansion behaviour from stack ....
#pad[PL #align(right)[PR]]

// ... block ...
#block(pad[PL #align(right)[PR]])

// ... and box.
#box(pad[PL #align(right)[PR]])

---
// Test that the pad node doesn't consume the whole region.

#page!(height: 6cm)

#align(left)[Before]
#pad(10pt, image("../../res/tiger.jpg"))
#align(right)[After]

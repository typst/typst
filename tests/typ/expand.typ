// Test fit/fill expansion.

---
#let right(body) = align(right, body)
#let pad(body) = pad(left: 10pt, right: 10pt, body)

// Top-level paragraph fills page, boxed paragraph only when width is fixed.
L #right[R] \
#box(width: 50pt)[L #right[R]] \
#box[L #right[R]] \

// Pad inherits expansion behaviour.
#pad[PL #right[PR]] \
#box(pad[PL #right[PR]])

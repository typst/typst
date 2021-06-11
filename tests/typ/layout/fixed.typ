// Test shrink-to-fit vs expand.

---
#let right(body) = align(right, body)
#let pad(body) = pad(left: 10pt, right: 10pt, body)

// Top-level paragraph fills page, boxed paragraph only when the width is set.
L #right[R] \
#rect(width: 50pt)[L #right[R]] \
#rect[L #right[R]]

// Pad inherits expansion behaviour.
#rect(pad[PL #right[PR]])
#pad[PL #right[PR]]

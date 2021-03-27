// Test forced line breaks.

---
// Directly after word.
Line\ Break

// Spaces around.
Line \ Break

// Directly before word does not work.
No \Break

\ Before

Multiple \ \ \

Times

---
#let linebreak() = [
    // Inside the old line break definition is still active.
    #circle(radius: 2pt, fill: #000) \
]

A \ B \ C

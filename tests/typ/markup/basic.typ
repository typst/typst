// Test basic markup.

---
#let linebreak() = [
    // Inside the old line break definition is still active.
    #circle(radius: 2pt, fill: #000) \
]

A \ B \ C

---
// Paragraph breaks don't exist!
#let parbreak() = [ ]

No more

paragraph breaks

for you!

---
The non-breaking~space does work.

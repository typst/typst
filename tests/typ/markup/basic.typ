// Test basic markup.

---
#let linebreak() = [
    // Inside the old line break definition is still active.
    #square(length: 3pt, fill: black) \
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

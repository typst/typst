// Test the `box` and `block` containers.

---
// Test box in paragraph.
A #box[B \ C] D.

// Test box with height.
Spaced \
#box(height: 0.5cm) \
Apart

---
// Test block over multiple pages.

#page!(height: 60pt)

First!
#block[
  But, soft! what light through yonder window breaks? It is the east, and Juliet
  is the sun.
]

---
// Test shrink-to-fit vs expand.

// Top-level paragraph fills page
L #align(right)[R]

// Block also fills page.
#block[
  L #align(right)[R]
]

// Boxed paragraph respects width.
#box(width: 50pt)[
  L #align(right)[R]
]

// Boxed paragraph without width doesn't expand.
#box[
  L #align(right)[R]
]

// Test the `box` and `block` containers.

---
// Test box in paragraph.
A #box[B \ C] D.

// Test box with height.
Spaced \
#box(height: 0.5cm) \
Apart

---
// Test block sizing.
#set page(height: 120pt)
#set block(spacing: 0pt)
#block(width: 90pt, height: 80pt, fill: red)[
  #block(width: 60%, height: 60%, fill: green)
  #block(width: 50%, height: 60%, fill: blue)
]

---
// Test box sizing with layoutable child.
#box(
  width: 50pt,
  height: 50pt,
  fill: yellow,
  path(
    fill: purple,
    (0pt, 0pt),
    (30pt, 30pt),
    (0pt, 30pt),
    (30pt, 0pt),
  ),
)

---
// Test fr box.
Hello #box(width: 1fr, rect(height: 0.7em, width: 100%)) World

---
// Test block over multiple pages.

#set page(height: 60pt)

First!

#block[
  But, soft! what light through yonder window breaks? It is the east, and Juliet
  is the sun.
]

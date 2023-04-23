// Test the `image` function.

---
// Test loading different image formats.

// Load an RGBA PNG image.
#image("/rhino.png")

// Load an RGB JPEG image.
#set page(height: 60pt)
#image("/tiger.jpg")

---
// Test configuring the size and fitting behaviour of images.

// Set width and height explicitly.
#box(image("/rhino.png", width: 30pt))
#box(image("/rhino.png", height: 30pt))

// Set width and height explicitly and force stretching.
#image("/monkey.svg", width: 100%, height: 20pt, fit: "stretch")

// Make sure the bounding-box of the image is correct.
#align(bottom + right, image("/tiger.jpg", width: 40pt, alt: "A tiger"))

---
// Test all three fit modes.
#set page(height: 50pt, margin: 0pt)
#grid(
  columns: (1fr, 1fr, 1fr),
  rows: 100%,
  gutter: 3pt,
  image("/tiger.jpg", width: 100%, height: 100%, fit: "contain"),
  image("/tiger.jpg", width: 100%, height: 100%, fit: "cover"),
  image("/monkey.svg", width: 100%, height: 100%, fit: "stretch"),
)

---
// Does not fit to remaining height of page.
#set page(height: 60pt)
Stuff
#image("/rhino.png")

---
// Test baseline.
A #box(image("/tiger.jpg", height: 1cm, width: 80%)) B

---
// Test advanced SVG features.
#image("/pattern.svg")

---
// Error: 8-29 file not found (searched at typ/visualize/path/does/not/exist)
#image("path/does/not/exist")

---
// Error: 8-21 unknown image format
#image("./image.typ")

---
// Error: 8-18 failed to parse svg: found closing tag 'g' instead of 'style' in line 4
#image("/bad.svg")

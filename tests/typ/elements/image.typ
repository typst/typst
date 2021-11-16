// Test the `image` function.

---
// Test loading different image formats.

// Load an RGBA PNG image.
#image("../../res/rhino.png")

// Load an RGB JPEG image.
#page(height: 60pt)
#image("../../res/tiger.jpg")

---
// Test configuring the size and fitting behaviour of images.

// Set width and height explicitly.
#image("../../res/rhino.png", width: 30pt)
#image("../../res/rhino.png", height: 30pt)

// Set width and height explicitly and force stretching.
#image("../../res/tiger.jpg", width: 100%, height: 20pt, fit: "stretch")

// Make sure the bounding-box of the image is correct.
#align(bottom, right)
#image("../../res/tiger.jpg", width: 40pt)

---
// Test all three fit modes.
#page(height: 50pt, margins: 0pt)
#grid(
  columns: 3,
  rows: 100%,
  gutter: 3pt,
  image("../../res/tiger.jpg", fit: "contain"),
  image("../../res/tiger.jpg", fit: "cover"),
  image("../../res/tiger.jpg", fit: "stretch"),
)

---
// Does not fit to remaining height of page.
#page(height: 60pt)
Stuff \
Stuff
#image("../../res/rhino.png")

---
// Error: 8-29 file not found
#image("path/does/not/exist")

---
// Error: 8-21 failed to load image (unknown image format)
#image("./image.typ")

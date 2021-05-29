// Test the `image` function.

---
// Test loading different image formats.

// Load an RGBA PNG image.
#image("../../res/rhino.png")
#pagebreak()

// Load an RGB JPEG image.
#image("../../res/tiger.jpg")

// Error: 8-29 file not found
#image("path/does/not/exist")

// Error: 8-20 failed to load image
#image("./font.typ")

---
// Test configuring the size and fitting behaviour of images.

// Fit to width of page.
#image("../../res/rhino.png")

// Fit to height of page.
#page(height: 40pt, image("../../res/rhino.png"))

// Set width explicitly.
#image("../../res/rhino.png", width: 50pt)

// Set height explicitly.
#image("../../res/rhino.png", height: 50pt)

// Set width and height explicitly and force stretching.
#image("../../res/rhino.png", width: 25pt, height: 50pt)

// Make sure the bounding-box of the image is correct.
#align(bottom, right, image("../../res/tiger.jpg", width: 60pt))

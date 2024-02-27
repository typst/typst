// Test the `image` function.

---
// Test loading different image formats.

// Load an RGBA PNG image.
#image("/assets/images/rhino.png")

// Load an RGB JPEG image.
#set page(height: 60pt)
#image("../../assets/images/tiger.jpg")

---
// Test configuring the size and fitting behaviour of images.

// Set width and height explicitly.
#box(image("/assets/images/rhino.png", width: 30pt))
#box(image("/assets/images/rhino.png", height: 30pt))

// Set width and height explicitly and force stretching.
#image("/assets/images/monkey.svg", width: 100%, height: 20pt, fit: "stretch")

// Make sure the bounding-box of the image is correct.
#align(bottom + right, image("/assets/images/tiger.jpg", width: 40pt, alt: "A tiger"))

---
// Test all three fit modes.
#set page(height: 50pt, margin: 0pt)
#grid(
  columns: (1fr, 1fr, 1fr),
  rows: 100%,
  gutter: 3pt,
  image("/assets/images/tiger.jpg", width: 100%, height: 100%, fit: "contain"),
  image("/assets/images/tiger.jpg", width: 100%, height: 100%, fit: "cover"),
  image("/assets/images/monkey.svg", width: 100%, height: 100%, fit: "stretch"),
)

---
// Does not fit to remaining height of page.
#set page(height: 60pt)
Stuff
#image("/assets/images/rhino.png")

---
// Test baseline.
A #box(image("/assets/images/tiger.jpg", height: 1cm, width: 80%)) B

---
// Test advanced SVG features.
#image("/assets/images/pattern.svg")

---
// Error: 8-29 file not found (searched at typ/visualize/path/does/not/exist)
#image("path/does/not/exist")

---
// Error: 2-22 unknown image format
#image("./image.typ")

---
// Error: 2-33 failed to parse SVG (found closing tag 'g' instead of 'style' in line 4)
#image("/assets/images/bad.svg")

---
// Test parsing from svg data
#image.decode(`<svg xmlns="http://www.w3.org/2000/svg" height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

---
// Error: 2-168 failed to parse SVG (missing root node)
#image.decode(`<svg height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

---
// Test format auto detect
#image.decode(read("/assets/images/tiger.jpg", encoding: none), width: 80%)

---
// Test format manual
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "jpg", width: 80%)

---
// Error: 2-91 failed to decode image (Format error decoding Png: Invalid PNG signature.)
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "png", width: 80%)

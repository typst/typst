// Test the `image` function.

---
// Test loading different image formats.

// Load an RGBA PNG image.
#image("/files/rhino.png")

// Load an RGB JPEG image.
#set page(height: 60pt)
#image("../../files/tiger.jpg")

---
// Test configuring the size and fitting behaviour of images.

// Set width and height explicitly.
#box(image("/files/rhino.png", width: 30pt))
#box(image("/files/rhino.png", height: 30pt))

// Set width and height explicitly and force stretching.
#image("/files/monkey.svg", width: 100%, height: 20pt, fit: "stretch")

// Make sure the bounding-box of the image is correct.
#align(bottom + right, image("/files/tiger.jpg", width: 40pt, alt: "A tiger"))

---
// Test all three fit modes.
#set page(height: 50pt, margin: 0pt)
#grid(
  columns: (1fr, 1fr, 1fr),
  rows: 100%,
  gutter: 3pt,
  image("/files/tiger.jpg", width: 100%, height: 100%, fit: "contain"),
  image("/files/tiger.jpg", width: 100%, height: 100%, fit: "cover"),
  image("/files/monkey.svg", width: 100%, height: 100%, fit: "stretch"),
)

---
// Does not fit to remaining height of page.
#set page(height: 60pt)
Stuff
#image("/files/rhino.png")

---
// Test baseline.
A #box(image("/files/tiger.jpg", height: 1cm, width: 80%)) B

---
// Test advanced SVG features.
#image("/files/pattern.svg")

---
// Error: 8-29 file not found (searched at typ/visualize/path/does/not/exist)
#image("path/does/not/exist")

---
// Error: 2-22 unknown image format
#image("./image.typ")

---
// Error: 2-25 failed to parse svg: found closing tag 'g' instead of 'style' in line 4
#image("/files/bad.svg")

---
// Test parsing from svg data
#image.decode(`<svg xmlns="http://www.w3.org/2000/svg" height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

---
// Error: 2-168 failed to parse svg: missing root node
#image.decode(`<svg height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

---
// Test format auto detect
#image.decode(read("/files/tiger.jpg", encoding: none), width: 80%)

---
// Test format manual
#image.decode(read("/files/tiger.jpg", encoding: none), format: "jpg", width: 80%)

---
// Error: 2-83 failed to decode image
#image.decode(read("/files/tiger.jpg", encoding: none), format: "png", width: 80%)

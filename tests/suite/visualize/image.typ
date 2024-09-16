// Test the `image` function.

--- image-png ---
// Load an RGBA PNG image.
#image("/assets/images/rhino.png")

--- image-jpg ---
// Load an RGB JPEG image.
#set page(height: 60pt)
#image("/assets/images/tiger.jpg")

--- image-sizing ---
// Test configuring the size and fitting behaviour of images.

// Set width and height explicitly.
#box(image("/assets/images/rhino.png", width: 30pt))
#box(image("/assets/images/rhino.png", height: 30pt))

// Set width and height explicitly and force stretching.
#image("/assets/images/monkey.svg", width: 100%, height: 20pt, fit: "stretch")

// Make sure the bounding-box of the image is correct.
#align(bottom + right, image("/assets/images/tiger.jpg", width: 40pt, alt: "A tiger"))

--- image-fit ---
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

--- image-jump-to-next-page ---
// Does not fit to remaining height of page.
#set page(height: 60pt)
Stuff
#image("/assets/images/rhino.png")

--- image-baseline-with-box ---
// Test baseline.
A #box(image("/assets/images/tiger.jpg", height: 1cm, width: 80%)) B

--- image-svg-complex ---
// Test advanced SVG features.
#image("/assets/images/pattern.svg")

--- image-svg-text ---
#set page(width: 250pt)

#figure(
  image("/assets/images/diagram.svg"),
  caption: [A textful diagram],
)

--- image-svg-text-font ---
#set page(width: 250pt)
#show image: set text(font: ("Roboto", "Noto Serif CJK SC"))

#figure(
  image("/assets/images/chinese.svg"),
  caption: [Bilingual text]
)

--- image-natural-dpi-sizing ---
// Test that images aren't upscaled.
// Image is just 48x80 at 220dpi. It should not be scaled to fit the page
// width, but rather max out at its natural size.
#image("/assets/images/f2t.jpg")

--- image-file-not-found ---
// Error: 8-29 file not found (searched at tests/suite/visualize/path/does/not/exist)
#image("path/does/not/exist")

--- image-bad-format ---
// Error: 2-22 unknown image format
#image("./image.typ")

--- image-bad-svg ---
// Error: 2-33 failed to parse SVG (found closing tag 'g' instead of 'style' in line 4)
#image("/assets/images/bad.svg")

--- image-decode-svg ---
// Test parsing from svg data
#image.decode(`<svg xmlns="http://www.w3.org/2000/svg" height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

--- image-decode-bad-svg ---
// Error: 2-168 failed to parse SVG (missing root node)
#image.decode(`<svg height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

--- image-decode-detect-format ---
// Test format auto detect
#image.decode(read("/assets/images/tiger.jpg", encoding: none), width: 80%)

--- image-decode-specify-format ---
// Test format manual
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "jpg", width: 80%)

--- image-decode-specify-wrong-format ---
// Error: 2-91 failed to decode image (Format error decoding Png: Invalid PNG signature.)
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "png", width: 80%)

--- issue-870-image-rotation ---
// Ensure that EXIF rotation is applied.
// https://github.com/image-rs/image/issues/1045
// File is from https://magnushoff.com/articles/jpeg-orientation/
#image("/assets/images/f2t.jpg", width: 10pt)

--- issue-measure-image ---
// Test that image measurement doesn't turn `inf / some-value` into 0pt.
#context {
  let size = measure(image("/assets/images/tiger.jpg"))
  test(size, (width: 1024pt, height: 670pt))
}

--- issue-2051-new-cm-svg ---
#set text(font: "New Computer Modern")
#image("/assets/images/diagram.svg")

--- issue-3733-dpi-svg ---
#set page(width: 200pt, height: 200pt, margin: 0pt)
#image("/assets/images/relative.svg")

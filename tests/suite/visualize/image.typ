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

--- image-svg-auto-detection ---
#image(bytes(
  ```
  <?xml version="1.0" encoding="utf-8"?>
  <!-- An SVG -->
  <svg width="200" height="150" xmlns="http://www.w3.org/2000/svg">
    <rect fill="red" stroke="black" x="25" y="25" width="150" height="100"/>
  </svg>
  ```.text
))

--- image-pixmap-rgb8 ---
#image(
  bytes((
    0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF,
    0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
    0x80, 0x80, 0x00, 0x00, 0x80, 0x80, 0x80, 0x00, 0x80,
  )),
  format: (
    encoding: "rgb8",
    width: 3,
    height: 3,
  ),
  width: 1cm,
)

--- image-pixmap-rgba8 ---
#image(
  bytes((
    0xFF, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0xFF,
    0xFF, 0x00, 0x00, 0x80, 0x00, 0xFF, 0x00, 0x80, 0x00, 0x00, 0xFF, 0x80,
    0xFF, 0x00, 0x00, 0x10, 0x00, 0xFF, 0x00, 0x10, 0x00, 0x00, 0xFF, 0x10,
  )),
  format: (
    encoding: "rgba8",
    width: 3,
    height: 3,
  ),
  width: 1cm,
)

--- image-pixmap-luma8 ---
#image(
  bytes(range(16).map(x => x * 16)),
  format: (
    encoding: "luma8",
    width: 4,
    height: 4,
  ),
  width: 1cm,
)

--- image-pixmap-lumaa8 ---
#image(
  bytes(range(16).map(x => (0x80, x * 16)).flatten()),
  format: (
    encoding: "lumaa8",
    width: 4,
    height: 4,
  ),
  width: 1cm,
)

--- image-scaling-methods ---
#let img(scaling) = image(
  bytes((
    0xFF, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00, 0xFF,
    0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x80,
    0x80, 0x80, 0x00, 0x00, 0x80, 0x80, 0x80, 0x00, 0x80,
  )),
  format: (
    encoding: "rgb8",
    width: 3,
    height: 3,
  ),
  width: 1cm,
  scaling: scaling,
)

#stack(
  dir: ltr,
  spacing: 4pt,
  img(auto),
  img("smooth"),
  img("pixelated"),
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
// Error: 2-37 unknown image format
#image("/assets/plugins/hello.wasm")

--- image-bad-svg ---
// Error: "/assets/images/bad.svg" 4:3 failed to parse SVG (found closing tag 'g' instead of 'style')
#image("/assets/images/bad.svg")

--- image-decode-svg ---
// Test parsing from svg data
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
#image.decode(`<svg xmlns="http://www.w3.org/2000/svg" height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

--- image-decode-bad-svg ---
// Error: 15-152 failed to parse SVG (missing root node at 1:1)
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
#image.decode(`<svg height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

--- image-decode-detect-format ---
// Test format auto detect
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
#image.decode(read("/assets/images/tiger.jpg", encoding: none), width: 80%)

--- image-decode-specify-format ---
// Test format manual
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "jpg", width: 80%)

--- image-decode-specify-wrong-format ---
// Error: 2-91 failed to decode image (Format error decoding Png: Invalid PNG signature.)
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "png", width: 80%)

--- image-pixmap-empty ---
// Error: 1:2-8:2 zero-sized images are not allowed
#image(
  bytes(()),
  format: (
    encoding: "rgb8",
    width: 0,
    height: 0,
  ),
)

--- image-pixmap-invalid-size ---
// Error: 1:2-8:2 pixel dimensions and pixel data do not match
#image(
  bytes((0x00, 0x00, 0x00)),
  format: (
    encoding: "rgb8",
    width: 16,
    height: 16,
  ),
)

--- image-pixmap-unknown-attribute ---
#image(
  bytes((0x00, 0x00, 0x00)),
  // Error: 1:11-6:4 unexpected key "stowaway", valid keys are "encoding", "width", and "height"
  format: (
    encoding: "rgb8",
    width: 1,
    height: 1,
    stowaway: "I do work here, promise",
  ),
)

--- image-pixmap-but-png-format ---
#image(
  bytes((0x00, 0x00, 0x00)),
  // Error: 1:11-5:4 expected "rgb8", "rgba8", "luma8", or "lumaa8"
  format: (
    encoding: "png",
    width: 1,
    height: 1,
  ),
)

--- image-png-but-pixmap-format ---
#image(
  read("/assets/images/tiger.jpg", encoding: none),
  // Error: 11-18 expected "png", "jpg", "gif", "webp", dictionary, "svg", or auto
  format: "rgba8",
)

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

--- image-exif-rotation ---
#let data = read("/assets/images/f2t.jpg", encoding: none)

#let rotations = range(1, 9)
#let rotated(v) = image(data.slice(0, 49) + bytes((v,)) + data.slice(50), width: 10pt)

#set page(width: auto)
#table(
  columns: rotations.len(),
  ..rotations.map(v => raw(str(v), lang: "typc")),
  ..rotations.map(rotated)
)

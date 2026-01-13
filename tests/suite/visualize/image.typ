// Test the `image` function.

--- image-png paged ---
// Load an RGBA PNG image.
#image("/assets/images/rhino.png")

--- image-jpg paged ---
// Load an RGB JPEG image.
#set page(height: 60pt)
#image("/assets/images/tiger.jpg")

--- image-jpg-html-base64 html ---
#image("/assets/images/f2t.jpg", alt: "The letter F")

--- image-sizing-html-css html ---
#image("/assets/images/f2t.jpg", width: 50%, alt: "width: 50%")
#image("/assets/images/f2t.jpg", width: 100pt, alt: "width: 100pt")
#image("/assets/images/f2t.jpg", width: 30% + 50pt, alt: "width: calc(30% + 50pt)")
#image("/assets/images/f2t.jpg", height: 75%, alt: "height: 75%")
#image("/assets/images/f2t.jpg", height: 80pt, alt: "height: 80pt")
#image("/assets/images/f2t.jpg", height: 20% + 40pt, alt: "height: calc(20% + 40pt)")

--- image-sizing paged ---
// Test configuring the size and fitting behaviour of images.

// Set width and height explicitly.
#box(image("/assets/images/rhino.png", width: 30pt))
#box(image("/assets/images/rhino.png", height: 30pt))

// Set width and height explicitly and force stretching.
#image("/assets/images/monkey.svg", width: 100%, height: 20pt, fit: "stretch")

// Make sure the bounding-box of the image is correct.
#align(bottom + right, image("/assets/images/tiger.jpg", width: 40pt, alt: "A tiger"))

--- image-fit paged ---
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

--- image-jump-to-next-page paged ---
// Does not fit to remaining height of page.
#set page(height: 60pt)
Stuff
#image("/assets/images/rhino.png")

--- image-baseline-with-box paged ---
// Test baseline.
A #box(image("/assets/images/tiger.jpg", height: 1cm, width: 80%)) B

--- image-svg-complex paged ---
// Test advanced SVG features.
#image("/assets/images/pattern.svg")

--- image-svg-text paged ---
#set page(width: 250pt)

#figure(
  image("/assets/images/diagram.svg"),
  caption: [A textful diagram],
)

--- image-svg-text-font paged ---
#set page(width: 250pt)
#show image: set text(font: ("Roboto", "Noto Serif CJK SC"))

#figure(
  image("/assets/images/chinese.svg"),
  caption: [Bilingual text]
)

--- image-svg-auto-detection paged ---
#image(bytes(
  ```
  <?xml version="1.0" encoding="utf-8"?>
  <!-- An SVG -->
  <svg width="200" height="150" xmlns="http://www.w3.org/2000/svg">
    <rect fill="red" stroke="black" x="25" y="25" width="150" height="100"/>
  </svg>
  ```.text
))

--- image-svg-linked-jpg1 paged ---
#set page(fill: gray)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg" height="80" width="48">
    <image href="../../../assets/images/f2t.jpg" />
    <circle r="32" cx="24" cy="40" fill="none" stroke="red" />
  </svg>
  ```.text
))

--- image-svg-linked-jpg2 paged ---
#set page(fill: gray)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg" height="80" width="48">
    <image href="file://../../../assets/images/f2t.jpg" />
    <circle r="32" cx="24" cy="40" fill="none" stroke="blue" />
  </svg>
  ```.text
))

--- image-svg-linked-many-formats paged ---
#set page(width: auto, height: auto, margin: 1pt)
#set text(1pt)
#image("../../../assets/images/linked.svg", width: 39pt)

--- image-svg-linked-file-not-found paged ---
// Error: 8-7:2 failed to load linked image do-not-add-image-with-this-name.png in SVG (file not found, searched at tests/suite/visualize/do-not-add-image-with-this-name.png)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg">
    <image href="do-not-add-image-with-this-name.png" />
  </svg>
  ```.text
))

--- image-svg-linked-url paged ---
// Error: 8-7:2 failed to load linked image https://somedomain.com/image.png in SVG (URLs are not allowed)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg">
    <image href="https://somedomain.com/image.png" />
  </svg>
  ```.text
))

--- image-svg-linked-pdf paged ---
// Error: 8-7:2 failed to load linked image ../../../assets/images/diagrams.pdf in SVG (PDF documents are not supported)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg">
    <image href="../../../assets/images/diagrams.pdf" />
  </svg>
  ```.text
))

--- image-svg-linked-csv paged ---
// Error: 8-7:2 failed to load linked image ../../../assets/data/bad.csv in SVG (unknown image format)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg">
    <image href="../../../assets/data/bad.csv" />
  </svg>
  ```.text
))

--- image-svg-linked-absolute1 paged ---
// Error: 8-7:2 failed to load linked image /home/user/foo.svg in SVG (absolute paths are not allowed)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg">
    <image href="/home/user/foo.svg" />
  </svg>
  ```.text
))

--- image-svg-linked-absolute2 paged ---
// Error: 8-7:2 failed to load linked image file:///home/user/foo.svg in SVG (absolute paths are not allowed)
#image(bytes(
  ```
  <svg xmlns="http://www.w3.org/2000/svg">
    <image href="file:///home/user/foo.svg" />
  </svg>
  ```.text
))

--- image-pixmap-rgb8 paged ---
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

--- image-pixmap-rgba8 paged ---
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

--- image-pixmap-luma8 paged ---
#image(
  bytes(range(16).map(x => x * 16)),
  format: (
    encoding: "luma8",
    width: 4,
    height: 4,
  ),
  width: 1cm,
)

--- image-pixmap-lumaa8 paged ---
#image(
  bytes(range(16).map(x => (0x80, x * 16)).flatten()),
  format: (
    encoding: "lumaa8",
    width: 4,
    height: 4,
  ),
  width: 1cm,
)

--- image-scaling-methods paged html ---
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

#let images = (
  img(auto),
  img("smooth"),
  img("pixelated"),
)

#context if target() == "html" {
  // TODO: Remove this once `stack` is supported in HTML export.
  html.div(
    style: "display: flex; flex-direction: row; gap: 4pt",
    images.join(),
  )
} else {
  stack(
    dir: ltr,
    spacing: 4pt,
    ..images,
  )
}

--- image-natural-dpi-sizing paged ---
// Test that images aren't upscaled.
// Image is just 48x80 at 220dpi. It should not be scaled to fit the page
// width, but rather max out at its natural size.
#image("/assets/images/f2t.jpg")

--- image-file-not-found paged ---
// Error: 8-29 file not found (searched at tests/suite/visualize/path/does/not/exist)
#image("path/does/not/exist")

--- image-bad-format paged ---
// Error: 2-37 unknown image format
#image("/assets/plugins/hello.wasm")

--- image-bad-svg paged ---
// Error: "/assets/images/bad.svg" 4:3 failed to parse SVG (found closing tag 'g' instead of 'style')
#image("/assets/images/bad.svg")

--- image-decode-svg paged ---
// Test parsing from svg data
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
// Hint: 8-14 it will be removed in Typst 0.15.0
#image.decode(`<svg xmlns="http://www.w3.org/2000/svg" height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

--- image-decode-bad-svg paged ---
// Error: 15-152 failed to parse SVG (missing root node at 1:1)
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
// Hint: 8-14 it will be removed in Typst 0.15.0
#image.decode(`<svg height="140" width="500"><ellipse cx="200" cy="80" rx="100" ry="50" style="fill:yellow;stroke:purple;stroke-width:2" /></svg>`.text, format: "svg")

--- image-decode-detect-format paged ---
// Test format auto detect
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
// Hint: 8-14 it will be removed in Typst 0.15.0
#image.decode(read("/assets/images/tiger.jpg", encoding: none), width: 80%)

--- image-decode-specify-format paged ---
// Test format manual
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
// Hint: 8-14 it will be removed in Typst 0.15.0
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "jpg", width: 80%)

--- image-decode-specify-wrong-format paged ---
// Error: 2-91 failed to decode image (Format error decoding Png: Invalid PNG signature.)
// Warning: 8-14 `image.decode` is deprecated, directly pass bytes to `image` instead
// Hint: 8-14 it will be removed in Typst 0.15.0
#image.decode(read("/assets/images/tiger.jpg", encoding: none), format: "png", width: 80%)

--- image-pixmap-empty paged ---
// Error: 1:2-8:2 zero-sized images are not allowed
#image(
  bytes(()),
  format: (
    encoding: "rgb8",
    width: 0,
    height: 0,
  ),
)

--- image-pixmap-invalid-size paged ---
// Error: 1:2-8:2 pixel dimensions and pixel data do not match
#image(
  bytes((0x00, 0x00, 0x00)),
  format: (
    encoding: "rgb8",
    width: 16,
    height: 16,
  ),
)

--- image-pixmap-unknown-attribute paged ---
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

--- image-pixmap-but-png-format paged ---
#image(
  bytes((0x00, 0x00, 0x00)),
  // Error: 1:11-5:4 expected "rgb8", "rgba8", "luma8", or "lumaa8"
  format: (
    encoding: "png",
    width: 1,
    height: 1,
  ),
)

--- image-png-but-pixmap-format paged ---
#image(
  read("/assets/images/tiger.jpg", encoding: none),
  // Error: 11-18 expected "png", "jpg", "gif", "webp", dictionary, "svg", "pdf", or auto
  format: "rgba8",
)

--- issue-measure-image paged ---
// Test that image measurement doesn't turn `inf / some-value` into 0pt.
#context {
  let size = measure(image("/assets/images/tiger.jpg"))
  test(size, (width: 1024pt, height: 670pt))
}

--- issue-2051-new-cm-svg paged ---
#set text(font: "New Computer Modern")
#image("/assets/images/diagram.svg")

--- issue-3733-dpi-svg paged ---
#set page(width: 200pt, height: 200pt, margin: 0pt)
#image("/assets/images/relative.svg")

--- image-exif-rotation paged ---
#let rotations = range(1, 9)
#let with-rotation(path, offset, v) = {
  let data = read(path, encoding: none)
  let modified = data.slice(0, offset) + bytes((v,)) + data.slice(offset + 1)
  image(modified, width: 10pt)
}

#set page(width: auto)
#table(
  columns: 1 + rotations.len(),
  table.header(
    [], ..rotations.map(v => raw(str(v), lang: "typc")),
  ),
  `PNG`, ..rotations.map(v => with-rotation("/assets/images/f2t.png", 0x85, v)),
  // JPEG has special handing in PDF export (no recoding, so instead we use a
  // transform to apply the orientation), so it's worth testing that separately.
  `JPEG`, ..rotations.map(v => with-rotation("/assets/images/f2t.jpg", 0x31, v)),
)

--- image-pdf-basic paged html ---
#image("/assets/images/star.pdf")

--- image-pdf-complex paged ---
#image("/assets/images/matplotlib.pdf")

--- image-pdf-multiple-pages paged ---
#image("/assets/images/diagrams.pdf", page: 1)
#image("/assets/images/diagrams.pdf", page: 3)
#image("/assets/images/diagrams.pdf", page: 2)

--- image-pdf-base14-fonts paged ---
// Test PDF base 14 fonts.
#image("/assets/images/base14-fonts.pdf")

--- image-pdf-invalid-page paged ---
// Error: 2-49 page 2 does not exist
// Hint: 2-49 the document only has 1 page
#image("/assets/images/matplotlib.pdf", page: 2)

--- issue-6869-image-zero-sized paged ---
// Primarily to ensure that it does not crash in PDF export.
#image("/assets/images/f2t.jpg", width: 0pt, height: 0pt)

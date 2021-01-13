// Test loading different image formats.

// Load an RGBA PNG image.
[image "res/rhino.png"]
[pagebreak]

// Load an RGB JPEG image.
[image "res/tiger.jpg"]

---
// Test configuring the size and fitting behaviour of images.

// Fit to width of page.
[image "res/rhino.png"]

// Fit to height of page.
[page width: 270pt][
    [image "res/rhino.png"]
]

// Set width explicitly.
[image "res/rhino.png", width: 50pt]

// Set height explicitly.
[image "res/rhino.png", height: 50pt]

// Set width and height explicitly and force stretching.
[image "res/rhino.png", width: 25pt, height: 50pt]

// Make sure the bounding-box of the image is correct.
[align bottom, right][
    [image "res/tiger.jpg"]
]

---
// Test error cases.
//
// ref: false
// error: 3:8-3:29 failed to load image
// error: 6:8-6:29 failed to load image

// File does not exist.
[image "path/does/not/exist"]

// File exists, but is no image.
[image "typ/image-error.typ"]

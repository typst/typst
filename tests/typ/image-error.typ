// Test error cases of the `image` function.

// compare-ref: false
// error: 8:9-8:30 failed to load image
// error: 11:9-11:30 failed to load image

// File does not exist.
[image: "path/does/not/exist"]

// File exists, but is no image.
[image: "typ/image-error.typ"]

// Test error cases of the `image` function.

// File does not exist.
[image: "path/does/not/exist"]

// File exists, but is no image.
[image: "typ/image-error.typ"]

// compare-ref: false
// error: 4:9-4:30 failed to load image
// error: 7:9-7:30 failed to load image

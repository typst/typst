// Test error cases of the `image` function.

// File does not exist.
[image "path/does/not/exist"]

// File exists, but is no image.
[image "typ/image-error.typ"]

// compare-ref: false
// error: 4:8-4:29 failed to load image
// error: 7:8-7:29 failed to load image

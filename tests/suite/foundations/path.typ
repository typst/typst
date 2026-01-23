--- path paged ---
#test(
  repr(path("hi/there.txt")),
  "path(\"/tests/suite/foundations/hi/there.txt\")",
)

--- path-escapes paged ---
// Error: 7-29 path `"../../../../file.txt"` would escape the project root
// Hint: 7-29 cannot access files outside of the project sandbox
// Hint: 7-29 you can adjust the project root with the `--root` argument
#read("../../../../file.txt")

--- path-backslash paged ---
// Error: 7-21 path must not contain a backslash
// Hint: 7-21 use forward slashes instead: `"to/file.txt"`
// Hint: 7-21 in earlier Typst versions, backslashes indicated path separators on Windows
// Hint: 7-21 this behavior is no longer supported as it is not portable
#path("to\\file.txt")

--- path-map paged ---
// Test that path can be mapped over an array of strings
#test(
  ("image.png", "projection.jpg", "transform.tiff").map(path),
  (path("/tests/suite/foundations/image.png"), path("/tests/suite/foundations/projection.jpg"), path("/tests/suite/foundations/transform.tiff"))
)

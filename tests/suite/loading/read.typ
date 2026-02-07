--- read-text eval ---
// Test reading plain text files
#let data = read("/assets/text/hello.txt")
#test(data, "Hello, world!\n")

--- read-file-not-found paged ---
// Error: 18-44 file not found (searched at assets/text/missing.txt)
#let data = read("/assets/text/missing.txt")

--- read-invalid-utf-8 paged ---
// Error: 18-40 failed to convert to string (file is not valid UTF-8 in assets/text/bad.txt:1:1)
#let data = read("/assets/text/bad.txt")

--- read-escapes paged ---
// Error: 7-29 path `"../../../../file.txt"` would escape the project root
// Hint: 7-29 cannot access files outside of the project sandbox
// Hint: 7-29 you can adjust the project root with the `--root` argument
#read("../../../../file.txt")

--- read-through-package eval ---
#import "@test/reader:0.1.0"

// Reads from the package.
#test(read(reader.hello-path), "Hello from package\n")
#test(reader.read-it("/hello.txt"), "Hello from package\n")

// Reads from the project.
#test(reader.read-it(path("/assets/text/hello.txt")), "Hello, world!\n")

--- read-from-project-in-package-fails paged ---
#import "@test/reader:0.1.0"

// Error: "tests/packages/reader-0.1.0/src/lib.typ" 2:24-2:25 file not found (searched at tests/packages/reader-0.1.0/assets/text/hello.txt)
#reader.read-it("/assets/text/hello.txt")

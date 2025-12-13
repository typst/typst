--- read-text paged ---
// Test reading plain text files
#let data = read("/assets/text/hello.txt")
#test(data, "Hello, world!\n")

--- read-file-not-found paged ---
// Error: 18-44 file not found (searched at assets/text/missing.txt)
#let data = read("/assets/text/missing.txt")

--- read-invalid-utf-8 paged ---
// Error: 18-40 failed to convert to string (file is not valid UTF-8 in assets/text/bad.txt:1:1)
#let data = read("/assets/text/bad.txt")

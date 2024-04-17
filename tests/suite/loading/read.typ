--- read-text ---
// Test reading plain text files
#let data = read("/assets/text/hello.txt")
#test(data, "Hello, world!\n")

--- read-file-not-found ---
// Error: 18-44 file not found (searched at assets/text/missing.txt)
#let data = read("/assets/text/missing.txt")

--- read-invalid-utf-8 ---
// Error: 18-40 file is not valid utf-8
#let data = read("/assets/text/bad.txt")

--- path-escapes paged ---
// Error: 7-29 path would escape the project root
#read("../../../../file.txt")

--- path-backslash paged ---
// Error: 7-21 path must not contain a backslash
#read("to\\file.txt")

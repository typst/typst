--- file-exists-text ---
// Test reading plain text files
#let data = file-exists("/assets/text/hello.txt")
#test(data, true)

--- file-exists-file-not-found ---
#let data = file-exists("/assets/text/missing.txt")
#test(data, false)

--- file-exists-invalid-utf-8 ---
#let data = file-exists("/assets/text/bad.txt")
#test(data, true)

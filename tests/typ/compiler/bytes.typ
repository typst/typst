// Test the bytes type.
// Ref: false

---
#let data = read("/files/rhino.png", encoding: none)
#test(data.len(), 232243)
#test(data.slice(0, count: 5), bytes((137, 80, 78, 71, 13)))
#test(str(data.slice(1, 4)), "PNG")
#test(repr(data), "bytes(232243)")

---
#test(str(bytes(range(0x41, 0x50))), "ABCDEFGHIJKLMNO")
#test(array(bytes("Hello")), (0x48, 0x65, 0x6C, 0x6C, 0x6F))

---
// Error: 8-14 expected string, array, or bytes, found dictionary
#bytes((a: 1))

---
// Error: 8-15 expected bytes or array, found string
#array("hello")

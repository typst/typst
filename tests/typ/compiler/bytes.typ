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
// Test addition and joining.
#test(bytes((1, 2)) + bytes(()), bytes((1, 2)))
#test(bytes((1, 2)) + bytes((3, 4)), bytes((1, 2, 3, 4)))
#test(bytes(()) + bytes((3, 4)), bytes((3, 4)))
#test(str({
  bytes("Hello")
  bytes((0x20,))
  bytes("World")
}), "Hello World")

---
// Error: 8-14 expected string, array, or bytes, found dictionary
#bytes((a: 1))

---
// Error: 8-15 expected bytes, array, or version, found string
#array("hello")

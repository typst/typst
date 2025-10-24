// Test the bytes type.

--- bytes-basic paged ---
#let data = read("/assets/images/rhino.png", encoding: none)
#test(data.len(), 232243)
#test(data.slice(0, count: 5), bytes((137, 80, 78, 71, 13)))
#test(str(data.slice(1, 4)), "PNG")
#test(repr(data), "bytes(232243)")

--- bytes-string-conversion paged ---
#test(str(bytes(range(0x41, 0x50))), "ABCDEFGHIJKLMNO")

--- bytes-array-conversion paged ---
#test(array(bytes("Hello")), (0x48, 0x65, 0x6C, 0x6C, 0x6F))

--- bytes-addition paged ---
// Test addition and joining.
#test(bytes((1, 2)) + bytes(()), bytes((1, 2)))
#test(bytes((1, 2)) + bytes((3, 4)), bytes((1, 2, 3, 4)))
#test(bytes(()) + bytes((3, 4)), bytes((3, 4)))

--- bytes-joining paged ---
#test(str({
  bytes("Hello")
  bytes((0x20,))
  bytes("World")
}), "Hello World")

--- bytes-bad-conversion-from-dict paged ---
// Error: 8-14 expected string, array, or bytes, found dictionary
#bytes((a: 1))

--- bytes-slice paged ---
// Test the `slice` method.
#test(bytes("abcd").slice(2), bytes("cd"))
#test(bytes("abcd").slice(0, 3), bytes("abc"))
#test(bytes("abcd").slice(1, -1), bytes("bc"))
#test(bytes("abcd").slice(3, 3), bytes(""))
#test(bytes("abcd").slice(3, 0), bytes(""))
#test(bytes("abcd").slice(-2), bytes("cd"))
#test(bytes("abcd").slice(-3, 2), bytes("b"))
#test(bytes("abcd").slice(-3, -1), bytes("bc"))
#test(bytes("abcd").slice(-2, -2), bytes(""))
#test(bytes("abcd").slice(1, count: 3), bytes("bcd"))
#test(bytes("abcd").slice(-3, count: 3), bytes("bcd"))
#test(bytes("abcd").slice(2, count: 0), bytes(""))

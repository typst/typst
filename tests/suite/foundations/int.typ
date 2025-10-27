--- int-base-alternative render ---
// Test numbers with alternative bases.
#test(0x10, 16)
#test(0b1101, 13)
#test(0xA + 0xa, 0x14)

--- int-base-binary-invalid render ---
// Error: 2-7 invalid binary number: 0b123
#0b123

--- int-base-hex-invalid render ---
// Error: 2-8 invalid hexadecimal number: 0x123z
#0x123z

--- int-constructor render ---
// Test conversion to numbers.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("150"), 150)
#test(int("-834"), -834)
#test(int("\u{2212}79"), -79)
#test(int(10 / 3), 3)
#test(int(-58.34), -58)
#test(int(decimal("92492.193848921")), 92492)
#test(int(decimal("-224.342211")), -224)

--- int-constructor-bad-type render ---
// Error: 6-10 expected integer, boolean, float, decimal, or string, found length
#int(10pt)

--- int-constructor-bad-value render ---
// Error: 6-12 invalid integer: nope
#int("nope")

--- int-constructor-float-too-large render ---
// Error: 6-27 number too large
#int(9223372036854775809.5)

--- int-constructor-float-too-large-min render ---
// Error: 6-28 number too large
#int(-9223372036854775809.5)

--- int-constructor-decimal-too-large render ---
// Error: 6-38 number too large
#int(decimal("9223372036854775809.5"))

--- int-constructor-decimal-too-large-min render ---
// Error: 6-39 number too large
#int(decimal("-9223372036854775809.5"))

--- int-signum render ---
// Test int `signum()`
#test(int(0).signum(), 0)
#test(int(1.0).signum(), 1)
#test(int(-1.0).signum(), -1)
#test(int(10.0).signum(), 1)
#test(int(-10.0).signum(), -1)

--- int-from-and-to-bytes render ---
// Test `int.from-bytes` and `int.to-bytes`.
#test(int.from-bytes(bytes(())), 0)
#test(int.from-bytes(bytes((1, 0, 0, 0, 0, 0, 0, 0)), endian: "little", signed: true), 1)
#test(int.from-bytes(bytes((1, 0, 0, 0, 0, 0, 0, 0)), endian: "big", signed: true), 72057594037927936)
#test(int.from-bytes(bytes((1, 0, 0, 0, 0, 0, 0, 0)), endian: "little", signed: false), 1)
#test(int.from-bytes(bytes((255,)), endian: "big", signed: true), -1)
#test(int.from-bytes(bytes((255,)), endian: "big", signed: false), 255)
#test(int.from-bytes((-1000).to-bytes(endian: "big", size: 5), endian: "big", signed: true), -1000)
#test(int.from-bytes((-1000).to-bytes(endian: "little", size: 5), endian: "little", signed: true), -1000)
#test(int.from-bytes(1000.to-bytes(endian: "big", size: 5), endian: "big", signed: true), 1000)
#test(int.from-bytes(1000.to-bytes(endian: "little", size: 5), endian: "little", signed: true), 1000)
#test(int.from-bytes(1000.to-bytes(endian: "little", size: 5), endian: "little", signed: false), 1000)

--- int-from-and-to-bytes-too-many render ---
// Error: 2-34 too many bytes to convert to a 64 bit number
#int.from-bytes(bytes((0,) * 16))

--- int-repr render ---
// Test the `repr` function with integers.
#test(repr(12), "12")
#test(repr(1234567890), "1234567890")
#test(repr(0123456789), "123456789")
#test(repr(0), "0")
#test(repr(-0), "0")
#test(repr(-1), "-1")
#test(repr(-9876543210), "-9876543210")
#test(repr(-0987654321), "-987654321")
#test(repr(4 - 8), "-4")

--- int-display render ---
// Test integers.
#12 \
#1234567890 \
#0123456789 \
#0 \
#(-0) \
#(-1) \
#(-9876543210) \
#(-0987654321) \
#(4 - 8)

--- issue-int-constructor render ---
// Test that integer -> integer conversion doesn't do a roundtrip through float.
#let x = 9223372036854775800
#test(type(x), int)
#test(int(x), x)

--- number-invalid-suffix render ---
// Error: 2-4 invalid number suffix: u
#1u

--- int-base-alternative eval ---
// Test numbers with alternative bases.
#test(0x10, 16)
#test(0b1101, 13)
#test(0xA + 0xa, 0x14)

--- int-base-binary-invalid eval ---
// Error: 2-7 invalid binary number: `0b123`
#0b123

--- int-base-octal-invalid eval ---
// Error: 2-11 invalid octal number: `0o1078970`
#0o1078970

--- int-base-hex-invalid eval ---
// Error: 2-9 invalid hexadecimal number: `0x123z4`
#0x123z4

--- int-base-hex-invalid-non-ascii eval ---
// Error: 9 expected comma
#(0xabcdéf)

--- int-base-empty eval ---
// Error: 2-4 expected a hexadecimal number
#0x
// Error: 2-4 expected an octal number
#0o
// Error: 2-4 expected a binary number
#0b

--- int-max eval ---
#test(int.max, 9223372036854775807)
#test(int.max, 0x7FFFFFFFFFFFFFFF)

--- int-min eval ---
#test(int.min, -1 - int.max)
#test(int.min, int("-9223372036854775808"))

--- int-bounds-max-overflow eval ---
// Error: 3-14 value is too large
#(int.max + 1)

--- int-bounds-min-underflow eval ---
// TODO: Change this to "too small".
// Error: 3-14 value is too large
#(int.min - 1)

--- int-bounds-max-plus-one eval ---
#test(type(9223372036854775808), float)

--- int-bounds-max-plus-one-hex eval ---
// Error: 2-20 invalid hexadecimal number: `0x8000000000000000`
#0x8000000000000000

--- int-bounds-max-plus-two eval ---
#test(type(9223372036854775809), float)

--- int-bounds-max-u64 eval ---
#test(type(18446744073709551615), float)

--- int-bounds-manual-min eval ---
#test(type(-9223372036854775808), float)

--- int-bounds-manual-min-hex eval ---
// Error: 4-22 invalid hexadecimal number: `0x8000000000000000`
#(-0x8000000000000000)

--- int-constructor eval ---
// Test conversion to numbers.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("0"), 0)
#test(int("+150"), 150)
#test(int("-834"), -834)
#test(int("beef", base: 16), 48879)
#test(int("-cAfFe", base: 16), -831486)
#test(int("10", base: 2), 2)
#test(int("644", base: 8), 420)
#test(int("\u{2212}79"), -79)
#test(int("9223372036854775807"), int.max)
#test(int("-9223372036854775808"), int.min)
#test(int("7FFFFFFFFFFFFFFF", base: 16), int.max)
#test(int("-8000000000000000", base: 16), int.min)
#test(int(10 / 3), 3)
#test(int(-58.34), -58)
#test(int(decimal("92492.193848921")), 92492)
#test(int(decimal("-224.342211")), -224)

--- int-constructor-bad-type eval ---
// Error: 6-10 expected integer, boolean, float, decimal, or string, found length
#int(10pt)

--- int-constructor-str-empty eval ---
// Error: 6-8 string must not be empty
#int("")

--- int-constructor-str-empty-based eval ---
// Error: 6-8 string must not be empty
#int("", base: 16)

--- int-constructor-bad-value eval ---
// Error: 6-12 string contains invalid digits
#int("nope")

--- int-constructor-bad-value-based eval ---
// Error: 6-11 string contains invalid digits for a base 3 integer
#int("123", base: 3)

--- int-constructor-base-with-non-string eval ---
// Error: 16-18 base is only supported for strings
#int(40, base: 16)

--- int-constructor-str-small-base eval ---
// Error: 17-18 base must be between 2 and 36
#int("0", base: 1)

--- int-constructor-str-large-base eval ---
// Error: 17-19 base must be between 2 and 36
#int("0", base: 42)

--- int-constructor-str-too-large eval ---
// Error: 6-27 integer value is too large
// Hint: 6-27 value does not fit into a signed 64-bit integer
// Hint: 6-27 try using a floating point number
#int("9223372036854775808")

--- int-constructor-str-too-small eval ---
// Error: 6-28 integer value is too small
// Hint: 6-28 value does not fit into a signed 64-bit integer
// Hint: 6-28 try using a floating point number
#int("-9223372036854775809")

--- int-constructor-float-too-large eval ---
// Error: 6-27 number too large
#int(9223372036854775809.5)

--- int-constructor-float-too-large-min eval ---
// Error: 6-28 number too large
#int(-9223372036854775809.5)

--- int-constructor-decimal-too-large eval ---
// Error: 6-38 number too large
#int(decimal("9223372036854775809.5"))

--- int-constructor-decimal-too-large-min eval ---
// Error: 6-39 number too large
#int(decimal("-9223372036854775809.5"))

--- int-signum eval ---
// Test int `signum()`
#test(int(0).signum(), 0)
#test(int(1.0).signum(), 1)
#test(int(-1.0).signum(), -1)
#test(int(10.0).signum(), 1)
#test(int(-10.0).signum(), -1)

--- int-from-and-to-bytes eval ---
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

--- int-from-and-to-bytes-too-many eval ---
// Error: 2-34 too many bytes to convert to a 64 bit number
#int.from-bytes(bytes((0,) * 16))

--- int-repr eval ---
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

--- int-display paged ---
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

--- issue-int-constructor eval ---
// Test that integer -> integer conversion doesn't do a roundtrip through float.
#let x = 9223372036854775800
#test(type(x), int)
#test(int(x), x)

--- number-invalid-suffix eval ---
// Error: 2-4 invalid number suffix: `u`
#1u

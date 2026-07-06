--- int-base-alternative eval ---
// Test numbers with alternative bases.
#test(0x10, 16)
#test(0b1101, 13)
#test(0xA + 0xa, 0x14)

--- int-base-binary-invalid eval ---
// Error: 2-7 integer contains digits that are not valid for a binary number
// Hint: 2-7 binary numbers only allow digits 0-1
// Hint: 5-7 the digits `2` and `3` are invalid
#0b123

--- int-base-octal-invalid eval ---
// Error: 2-11 integer contains digits that are not valid for an octal number
// Hint: 2-11 octal numbers only allow digits 0-7
// Hint: 7-9 the digits `8` and `9` are invalid
#0o1078970

--- int-base-hex-invalid eval ---
// Error: 2-9 integer contains digits that are not valid for a hexadecimal number
// Hint: 2-9 hexadecimal numbers only allow digits 0-9, a-f, A-F
// Hint: 7-8 the digit `z` is invalid
#0x123z4

--- int-base-binary-invalid-long eval ---
// Error: 2-14 integer contains digits that are not valid for a binary number
// Hint: 2-14 binary numbers only allow digits 0-1
// Hint: 6-14 the digits `2`, `3`, `4`, `5`, `6`, `7`, `8`, and `9` are invalid
#0b0123456789

--- int-base-octal-invalid-long eval ---
// Error: 2-14 integer contains digits that are not valid for an octal number
// Hint: 2-14 octal numbers only allow digits 0-7
// Hint: 12-14 the digits `8` and `9` are invalid
#0o0123456789

--- int-base-hex-invalid-long eval ---
// Error: 2-56 integer contains digits that are not valid for a hexadecimal number
// Hint: 2-56 hexadecimal numbers only allow digits 0-9, a-f, A-F
// Hint: 10-56 the digits `g`, `h`, `i`, `j`, `k`, `l`, `m`, `n`, `o`, `p`, `q`, `r`, `s`, `t`, `u`, `v`, `w`, `y`, `z`, `G`, `H`, `I`, `J`, `K`, `L`, `M`, `N`, `O`, `P`, `Q`, `R`, `S`, `T`, `U`, `V`, `W`, `X`, `Y`, and `Z` are invalid
#0xabcdefghijklmnopqrstuvwkyzABCDEFGHIJKLMNOPQRSTUVWXYZ

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
// Error: 2-21 integer value is too large
// Hint: 2-21 value does not fit into a signed 64-bit integer
// Hint: 2-21 a floating point number could approximately represent this value
// Hint: 2-21 you can use a floating point number by appending a dot: `9223372036854775808.`
#9223372036854775808

--- int-bounds-max-plus-one-hex eval ---
// Error: 2-20 integer value is too large
// Hint: 2-20 value does not fit into a signed 64-bit integer
#0x8000000000000000

--- int-bounds-max-plus-two eval ---
// Error: 2-21 integer value is too large
// Hint: 2-21 value does not fit into a signed 64-bit integer
// Hint: 2-21 a floating point number could approximately represent this value
// Hint: 2-21 you can use a floating point number by appending a dot: `9223372036854775809.`
#9223372036854775809

--- int-bounds-max-u64 eval ---
// Error: 2-22 integer value is too large
// Hint: 2-22 value does not fit into a signed 64-bit integer
// Hint: 2-22 a floating point number could approximately represent this value
// Hint: 2-22 you can use a floating point number by appending a dot: `18446744073709551615.`
#18446744073709551615

--- int-bounds-manual-min-no-space eval ---
// Error: 3-23 cannot write minimum integer manually
// Hint: 3-23 Typst integers are always initially positive
// Hint: 3-23 2^63 does not fit into a signed 64-bit integer
// Hint: 3-23 try writing `int.min`
#(-9223372036854775808)

--- int-bounds-manual-min-space eval ---
// Error: 3-24 cannot write minimum integer manually
// Hint: 3-24 Typst integers are always initially positive
// Hint: 3-24 2^63 does not fit into a signed 64-bit integer
// Hint: 3-24 try writing `int.min`
#(- 9223372036854775808)

--- int-bounds-manual-min-newline eval ---
// Error: 1:3-2:23 cannot write minimum integer manually
// Hint: 1:3-2:23 Typst integers are always initially positive
// Hint: 1:3-2:23 2^63 does not fit into a signed 64-bit integer
// Hint: 1:3-2:23 try writing `int.min`
#(-
   9223372036854775808)

--- int-bounds-manual-min-hex eval ---
// Error: 3-22 cannot write minimum integer manually
// Hint: 3-22 Typst integers are always initially positive
// Hint: 3-22 2^63 does not fit into a signed 64-bit integer
// Hint: 3-22 try writing `int.min`
#(-0x8000000000000000)

--- int-bounds-min-minus-one eval ---
// Error: 3-23 integer value is too small
// Hint: 3-23 value does not fit into a signed 64-bit integer
// Hint: 3-23 a floating point number could approximately represent this value
// Hint: 3-23 you can use a floating point number by appending a dot: `9223372036854775809.`
#(-9223372036854775809)

--- int-bounds-invalid-no-syntax-error eval ---
// Some integer syntax errors happen in the AST, so we won't actually error if
// the int isn't evaluated.
#let _ = () => 9223372036854775808
#let _ = () => -9223372036854775808
#let _ = () => 0b123

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

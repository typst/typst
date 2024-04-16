--- int-base-alternative ---
// Test numbers with alternative bases.
#test(0x10, 16)
#test(0b1101, 13)
#test(0xA + 0xa, 0x14)

--- int-base-binary-invalid ---
// Error: 2-7 invalid binary number: 0b123
#0b123

--- int-base-hex-invalid ---
// Error: 2-8 invalid hexadecimal number: 0x123z
#0x123z

--- int-constructor ---
// Test conversion to numbers.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("150"), 150)
#test(int("-834"), -834)
#test(int("\u{2212}79"), -79)
#test(int(10 / 3), 3)

--- int-constructor-bad-type ---
// Error: 6-10 expected integer, boolean, float, or string, found length
#int(10pt)

--- int-constructor-bad-value ---
// Error: 6-12 invalid integer: nope
#int("nope")

--- int-signum ---
// Test int `signum()`
#test(int(0).signum(), 0)
#test(int(1.0).signum(), 1)
#test(int(-1.0).signum(), -1)
#test(int(10.0).signum(), 1)
#test(int(-10.0).signum(), -1)

--- int-repr ---
// Test the `repr` function with integers.
#repr(12) \
#repr(1234567890) \
#repr(0123456789) \
#repr(0) \
#repr(-0) \
#repr(-1) \
#repr(-9876543210) \
#repr(-0987654321) \
#repr(4 - 8)

--- int-display ---
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

--- issue-int-constructor ---
// Test that integer -> integer conversion doesn't do a roundtrip through float.
#let x = 9223372036854775800
#test(type(x), int)
#test(int(x), x)

--- number-invalid-suffix ---
// Error: 2-4 invalid number suffix: u
#1u

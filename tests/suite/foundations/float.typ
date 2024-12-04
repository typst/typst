--- float-constructor ---
#test(float(10), 10.0)
#test(float(50% * 30%), 0.15)
#test(float("31.4e-1"), 3.14)
#test(float("31.4e\u{2212}1"), 3.14)
#test(float("3.1415"), 3.1415)
#test(float("-7654.321"), -7654.321)
#test(float("\u{2212}7654.321"), -7654.321)
#test(float(decimal("4.89")), 4.89)
#test(float(decimal("3.1234567891234567891234567891")), 3.123456789123457)
#test(float(decimal("79228162514264337593543950335")), 79228162514264340000000000000.0)
#test(float(decimal("-79228162514264337593543950335")), -79228162514264340000000000000.0)
#test(type(float(10)), float)

--- float-constructor-bad-type ---
// Error: 8-13 expected float, boolean, integer, decimal, ratio, or string, found type
#float(float)

--- float-constructor-bad-value ---
// Error: 8-15 invalid float: 1.2.3
#float("1.2.3")

--- float-is-nan ---
// Test float `is-nan()`.
#test(float(float.nan).is-nan(), true)
#test(float(10).is-nan(), false)
#test(float(calc.inf).is-nan(), false)
#test(float(-calc.inf).is-nan(), false)

--- float-is-infinite ---
// Test float `is-infinite()`.
#test(float(calc.inf).is-infinite(), true)
#test(float(-calc.inf).is-infinite(), true)
#test(float(10).is-infinite(), false)
#test(float(-10).is-infinite(), false)
#test(float(float.nan).is-infinite(), false)

--- float-signum ---
// Test float `signum()`
#test(float(0.0).signum(), 1.0)
#test(float(1.0).signum(), 1.0)
#test(float(-1.0).signum(), -1.0)
#test(float(10.0).signum(), 1.0)
#test(float(-10.0).signum(), -1.0)
#test(float(calc.inf).signum(), 1.0)
#test(float(-calc.inf).signum(), -1.0)
#test(float(float.nan).signum().is-nan(), true)

--- float-from-and-to-bytes ---
// Test float `from-bytes()` and `to-bytes()`.
#test(float.from-bytes(bytes((0, 0, 0, 0, 0, 0, 240, 63))), 1.0)
#test(float.from-bytes(bytes((63, 240, 0, 0, 0, 0, 0, 0)), endian: "big"), 1.0)
#test(1.0.to-bytes(), bytes((0, 0, 0, 0, 0, 0, 240, 63)))
#test(1.0.to-bytes(endian: "big"), bytes((63, 240, 0, 0, 0, 0, 0, 0)))

#test(float.from-bytes(bytes((0, 0, 32, 64))), 2.5)
#test(float.from-bytes(bytes((64, 32, 0, 0)), endian: "big"), 2.5)
#test(2.5.to-bytes(size: 4), bytes((0, 0, 32, 64)))
#test(2.5.to-bytes(size: 4, endian: "big"), bytes((64, 32, 0, 0)))

--- float-from-bytes-bad-length ---
// Error: 2-54 bytes must have a length of 4 or 8
#float.from-bytes(bytes((0, 0, 0, 0, 0, 0, 0, 1, 0)))

--- float-repr ---
// Test the `repr` function with floats.
#test(repr(12.0), "12.0")
#test(repr(3.14), "3.14")
#test(repr(1234567890.0), "1234567890.0")
#test(repr(0123456789.0), "123456789.0")
#test(repr(0.0), "0.0")
#test(repr(-0.0), "-0.0")
#test(repr(-1.0), "-1.0")
#test(repr(-9876543210.0), "-9876543210.0")
#test(repr(-0987654321.0), "-987654321.0")
#test(repr(-3.14), "-3.14")
#test(repr(4.0 - 8.0), "-4.0")
#test(repr(float.inf), "float.inf")
#test(repr(-float.inf), "-float.inf")
#test(repr(float.nan), "float.nan")

--- float-display ---
// Test floats.
#12.0 \
#3.14 \
#1234567890.0 \
#0123456789.0 \
#0.0 \
#(-0.0) \
#(-1.0) \
#(-9876543210.0) \
#(-0987654321.0) \
#(-3.14) \
#(4.0 - 8.0) \
#float.inf \
#(-float.inf) \
#float.nan

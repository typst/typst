--- float-constructor ---
#test(float(10), 10.0)
#test(float(50% * 30%), 0.15)
#test(float("31.4e-1"), 3.14)
#test(float("31.4e\u{2212}1"), 3.14)
#test(float("3.1415"), 3.1415)
#test(float("-7654.321"), -7654.321)
#test(float("\u{2212}7654.321"), -7654.321)
#test(type(float(10)), float)

--- float-constructor-bad-type ---
// Error: 8-13 expected float, boolean, integer, ratio, or string, found type
#float(float)

--- float-constructor-bad-value ---
// Error: 8-15 invalid float: 1.2.3
#float("1.2.3")

--- float-is-nan ---
// Test float `is-nan()`.
#test(float(calc.nan).is-nan(), true)
#test(float(10).is-nan(), false)

--- float-is-infinite ---
// Test float `is-infinite()`.
#test(float(calc.inf).is-infinite(), true)
#test(float(-calc.inf).is-infinite(), true)
#test(float(10).is-infinite(), false)
#test(float(-10).is-infinite(), false)

--- float-signum ---
// Test float `signum()`
#test(float(0.0).signum(), 1.0)
#test(float(1.0).signum(), 1.0)
#test(float(-1.0).signum(), -1.0)
#test(float(10.0).signum(), 1.0)
#test(float(-10.0).signum(), -1.0)
#test(float(calc.nan).signum().is-nan(), true)

--- float-repr ---
// Test the `repr` function with floats.
#repr(12.0) \
#repr(3.14) \
#repr(1234567890.0) \
#repr(0123456789.0) \
#repr(0.0) \
#repr(-0.0) \
#repr(-1.0) \
#repr(-9876543210.0) \
#repr(-0987654321.0) \
#repr(-3.14) \
#repr(4.0 - 8.0)

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
#(4.0 - 8.0)

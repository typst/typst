--- decimal-constructor ---
#test(decimal(10), decimal("10.0"))
#test(decimal("\u{2212}7654.321"), decimal("-7654.321"))
#test(type(decimal(10)), decimal)

--- decimal-constructor-bad-type ---
// Error: 10-17 expected integer or string, found type
#decimal(decimal)

--- decimal-constructor-bad-value ---
// Error: 10-17 invalid decimal: 1.2.3
#decimal("1.2.3")

--- decimal-repr ---
// Test the `repr` function with decimals.
#repr(decimal("12.0")) \
#repr(decimal("3.14")) \
#repr(decimal("1234567890.0")) \
#repr(decimal("0123456789.0")) \
#repr(decimal("0.0")) \
#repr(decimal("-0.0")) \
#repr(decimal("-1.0")) \
#repr(decimal("-9876543210.0")) \
#repr(decimal("-0987654321.0")) \
#repr(decimal("-3.14")) \
#repr(decimal("4.0") - decimal("8.0")) \

--- decimal-display ---
// Test decimals.
#decimal("12.0") \
#decimal("3.14") \
#decimal("1234567890.0") \
#decimal("0123456789.0") \
#decimal("0.0") \
#decimal("-0.0") \
#decimal("-1.0") \
#decimal("-9876543210.0") \
#decimal("-0987654321.0") \
#decimal("-3.14") \
#(decimal("4.0") - decimal("8.0"))

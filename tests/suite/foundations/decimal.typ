--- decimal-constructor ---
#test(decimal(10), decimal("10.0"))
#test(decimal("-7654.321"), decimal("-7654.321"))
#test(decimal("\u{2212}7654.321"), decimal("-7654.321"))
#test(decimal({ 3.141592653 }), decimal("3.141592653000000012752934707"))
#test(decimal({ -3.141592653 }), decimal("-3.141592653000000012752934707"))
#test(decimal(decimal(3)), decimal("3.0"))
#test(type(decimal(10)), decimal)

--- decimal-constructor-bad-type ---
// Error: 10-17 expected integer, float, string, or decimal, found type
#decimal(decimal)

--- decimal-constructor-bad-value ---
// Error: 10-17 invalid decimal: 1.2.3
#decimal("1.2.3")

--- decimal-constructor-float-literal ---
// Warning: 18-25 creating a decimal using imprecise float literal
// Hint: 18-25 use a string in the decimal constructor to avoid loss of precision: `decimal("1.32523")`
#let _ = decimal(1.32523)

--- decimal-constructor-float-inf ---
// Error: 10-19 float is not a valid decimal: float.inf
#decimal(float.inf)

--- decimal-constructor-float-negative-inf ---
// Error: 10-20 float is not a valid decimal: -float.inf
#decimal(-float.inf)

--- decimal-constructor-float-nan ---
// Error: 10-19 float is not a valid decimal: float.nan
#decimal(float.nan)

--- decimal-scale-is-observable ---
// Ensure equal decimals with different scales produce different strings.
#let f1(x) = str(x)
#let f2(x) = f1(x)
#test(f2(decimal("3.140")), "3.140")
#test(f2(decimal("3.14000")), "3.14000")

--- decimal-repr ---
// Test the `repr` function with decimals.
#test(repr(decimal("12.0")), "decimal(\"12.0\")")
#test(repr(decimal("3.14")), "decimal(\"3.14\")")
#test(repr(decimal("1234567890.0")), "decimal(\"1234567890.0\")")
#test(repr(decimal("0123456789.0")), "decimal(\"123456789.0\")")
#test(repr(decimal("0.0")), "decimal(\"0.0\")")
#test(repr(decimal("-0.0")), "decimal(\"0.0\")")
#test(repr(decimal("-1.0")), "decimal(\"-1.0\")")
#test(repr(decimal("-9876543210.0")), "decimal(\"-9876543210.0\")")
#test(repr(decimal("-0987654321.0")), "decimal(\"-987654321.0\")")
#test(repr(decimal("-3.14")), "decimal(\"-3.14\")")
#test(repr(decimal("-3.9191919191919191919191919195")), "decimal(\"-3.9191919191919191919191919195\")")
#test(repr(decimal("5.0000000000")), "decimal(\"5.0000000000\")")
#test(repr(decimal("4.0") - decimal("8.0")), "decimal(\"-4.0\")")

--- decimal-display ---
// Test decimals.
#set page(width: auto)
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
#decimal("-3.9191919191919191919191919195") \
#decimal("5.0000000000") \
#(decimal("4.0") - decimal("8.0"))

--- decimal-display-round ---
// Display less digits.
#calc.round(decimal("-3.9191919191919191919191919195"), digits: 4) \
#calc.round(decimal("5.0000000000"), digits: 4)

--- decimal-expected-float-error ---
// Error: 11-25 expected integer, float, or angle, found decimal
// Hint: 11-25 if loss of precision is acceptable, explicitly cast the decimal to a float with `float(value)`
#calc.sin(decimal("1.1"))

--- decimal-expected-integer-error ---
// Error: 11-25 expected integer, found decimal
#calc.odd(decimal("1.1"))

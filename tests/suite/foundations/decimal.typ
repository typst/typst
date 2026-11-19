--- decimal-constructor eval ---
#test(decimal(10), 10.0d)
#test(decimal("-7654.321"), -7654.321d)
#test(decimal({ 3.141592653 }), 3.141592653000000012752934707d)
#test(decimal({ -3.141592653 }), -3.141592653000000012752934707d)
#test(decimal(decimal(3)), 3.0d)
#test(decimal(true), 1.0d)
#test(type(decimal(10)), decimal)

--- decimal-constructor-minus-sign eval ---
// Test the Unicode minus sign: `\u{2212}`.
#let n = "1.23"
#let negative = decimal("-" + n)
#test(negative, decimal("\u{2212}" + n))
#test(negative, eval("-" + n + "d"))
#test(negative, eval("\u{2212}" + n + "d"))
#test(-negative, eval("\u{2212}\u{2212}" + n + "d"))
#test(negative, eval("\u{2212}-\u{2212}" + n + "d"))

--- decimal-constructor-bad-type eval ---
// Error: 10-17 expected decimal, integer, boolean, float, or string, found type
#decimal(decimal)

--- decimal-constructor-bad-string eval ---
// Error: 10-17 invalid decimal: `1.2.3`
#decimal("1.2.3")

--- decimal-constructor-string-suffix-hint eval ---
// Error: 10-17 invalid decimal: `1.23d`
// Hint: 10-17 try removing the `d` suffix: `1.23`
#decimal("1.23d")

--- decimal-constructor-string-suffix-no-hint eval ---
// We don't hint if the string would still be invalid without the suffix.
// Error: 10-18 invalid decimal: `1.2.3d`
#decimal("1.2.3d")

--- decimal-constructor-float-inf eval ---
// Error: 10-19 float is not a valid decimal: `float.inf`
#decimal(float.inf)

--- decimal-constructor-float-negative-inf eval ---
// Error: 10-20 float is not a valid decimal: `-float.inf`
#decimal(-float.inf)

--- decimal-constructor-float-nan eval ---
// Error: 10-19 float is not a valid decimal: `float.nan`
#decimal(float.nan)

--- decimal-constructor-float-too-large eval ---
// Error: 10-15 float is not a valid decimal: `1e308`
#decimal(1e308)

--- decimal-constructor-float-literal-inf eval ---
// Error: 10-15 float is not a valid decimal: `float.inf`
#decimal(1e309)

--- decimal-constructor-float-warning eval ---
// Warn for imprecise decimal conversions from floats.
// The warning only works for literal float syntax.
#let _ = decimal({ 1.32523 })
// It also excludes negatives since we don't have negative float literals.
#let _ = decimal(-1.32523)
// Warning: 18-25 creating a decimal using imprecise float literal
// Hint: 18-25 use a decimal literal to avoid loss of precision: `1.32523d`
#let _ = decimal(1.32523)
// Warning: 18-24 creating a decimal using imprecise float literal
// Hint: 18-24 use a decimal literal to avoid loss of precision: `0.1d`
#let _ = decimal(1.0e-1)
// Warning: 18-21 creating a decimal using imprecise float literal
// Hint: 18-21 use a decimal literal to avoid loss of precision: `100d`
#let _ = decimal(1E2)

--- decimal-constructor-float-warning-indirect eval ---
// The warning works through only some indirect calls based on value spans.
#let indirect-a(x) = decimal(x)
#let indirect-b = decimal
#let _ = indirect-a(1.32523)
// Warning: 21-28 creating a decimal using imprecise float literal
// Hint: 21-28 use a decimal literal to avoid loss of precision: `1.32523d`
#let _ = indirect-b(1.32523)

--- decimal-literal-edge-cases eval ---
// Error: 2-4 invalid number suffix: `D`
#1D
#1.d // Field access syntax will error in eval but not during parsing.
#1.e2d
// Error: 2-10 decimal literals cannot use exponent syntax
// Hint: 2-10 try expanding the exponent: `12300d`
#1.23e+4d
// Error: 2-8 decimal literals cannot use exponent syntax
// Hint: 2-8 try expanding the exponent: `100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000d`
#1e308d
// Error: 2-8 decimal literals cannot use exponent syntax
#1e309d
// Error: 2-8 invalid floating point number: `1.0E-`
#1.0E-d
// Error: 2-8 invalid floating point number: `1.23e`
#1.23ed
// Error: 2-5 invalid floating point number: `1E`
#1Ed
// Error: 2-7 invalid floating point number: `1.0E`
#1.0Ed
// Error: 2-11 invalid floating point number: `1.23e`
// Hint: 2-11 invalid number suffix: `d456`
#1.23ed456

--- decimal-scale-is-observable eval ---
// Ensure equal decimals with different scales produce different strings.
#let f1(x) = str(x)
#let f2(x) = f1(x)
#test(f2(3.140d), "3.140")
#test(f2(3.14000d), "3.14000")

--- decimal-repr eval ---
// Test the `repr` function with decimals.
#test(repr(12.0d), "12.0d")
#test(repr(3.14d), "3.14d")
#test(repr(1234567890.0d), "1234567890.0d")
#test(repr(0123456789.0d), "123456789.0d")
#test(repr(0.0d), "0.0d")
#test(repr(-1.0d), "-1.0d")
#test(repr(-9876543210.0d), "-9876543210.0d")
#test(repr(-0987654321.0d), "-987654321.0d")
#test(repr(-3.14d), "-3.14d")
#test(repr(-3.9191919191919191919191919195d), "-3.9191919191919191919191919195d")
#test(repr(5.0000000000d), "5.0000000000d")
#test(repr(4.0d - 8.0d), "-4.0d")

--- decimal-repr-negative-zero eval ---
// The `repr` for positive vs. negative zero unfortunately differs, but it is
// not worth fixing on Typst's side (hiding the underlying value from users
// could be _very_ confusing).
#let _ = quote(
  block: true,
  attribution: [https://github.com/paupino/rust-decimal/issues/799],
)[
  We normally try to stick to just `0` actually and not have the concept of a
  negative zero, but we do allow it to happen when to do otherwise would incur
  additional costs (e.g. zero checks on negation).
]
// First, note that negative and positive zero do compare equal:
#test(-0d, 0d)
// Now we compare the repr.
#let pos-repr = "0d"
#let neg-repr = "-0d"
#test(pos-repr, repr(0d))
// Negating a decimal can make a negative zero.
#test(neg-repr, repr(-0d))
// But most other arithmetic does not.
#test(pos-repr, repr(0 * -1d))
#test(pos-repr, repr(0d * -1))
#test(pos-repr, repr(-0d * 1))
#test(pos-repr, repr(calc.pow(-0d, 1)))
// The string constructor never makes a negative zero.
#test(pos-repr, repr(decimal("0")))
#test(pos-repr, repr(decimal("-0")))

--- decimal-display paged ---
// Test decimals.
#set page(width: auto)
#12.0d \
#3.14d \
#1234567890.0d \
#0123456789.0d \
#0.0d \
#(-0.0d) \
#(-1.0d) \
#(-9876543210.0d) \
#(-0987654321.0d) \
#(-3.14d) \
#(-3.9191919191919191919191919195d) \
#5.0000000000d \
#(4.0d - 8.0d)

--- decimal-display-round paged ---
// Display less digits.
#calc.round(-3.9191919191919191919191919195d, digits: 4) \
#calc.round(5.0000000000d, digits: 4)

--- decimal-expected-float-error eval ---
// Error: 11-15 expected integer, float, or angle, found decimal
// Hint: 11-15 if loss of precision is acceptable, explicitly cast the decimal to a float with `float(value)`
#calc.sin(1.1d)

--- decimal-expected-integer-error eval ---
// Error: 11-15 expected integer, found decimal
#calc.odd(1.1d)

--- calc-round ---
#test(calc.round(calc.e, digits: 2), 2.72)
#test(calc.round(calc.pi, digits: 2), 3.14)
#test(type(calc.round(3.1415, digits: 2)), float)
#test(type(calc.round(5, digits: 2)), int)
#test(type(calc.round(decimal("3.1415"), digits: 2)), decimal)

--- calc-round-large-inputs ---
#test(calc.round(31114, digits: 4000000000), 31114)
#test(calc.round(9223372036854775807, digits: 12), 9223372036854775807)
#test(calc.round(238959235.129590203, digits: 4000000000), 238959235.129590203)
#test(calc.round(1.7976931348623157e+308, digits: 12), 1.7976931348623157e+308)
#test(calc.round(decimal("238959235.129590203"), digits: 4000000000), decimal("238959235.129590203"))
#test(calc.round(decimal("79228162514264337593543950335"), digits: 12), decimal("79228162514264337593543950335"))

--- calc-round-negative-digits ---
// Error: 29-31 number must be at least zero
#calc.round(243.32, digits: -2)

--- calc-abs ---
// Test the `abs` function.
#test(calc.abs(-3), 3)
#test(calc.abs(3), 3)
#test(calc.abs(-0.0), 0.0)
#test(calc.abs(0.0), -0.0)
#test(calc.abs(-3.14), 3.14)
#test(calc.abs(50%), 50%)
#test(calc.abs(-25%), 25%)
#test(calc.abs(decimal("4932.493249324932")), decimal("4932.493249324932"))
#test(calc.abs(decimal("-12402.593295932041")), decimal("12402.593295932041"))

--- calc-abs-bad-type ---
// Error: 11-22 expected integer, float, length, angle, ratio, fraction, or decimal, found string
#calc.abs("no number")

--- calc-even-and-odd ---
// Test the `even` and `odd` functions.
#test(calc.even(2), true)
#test(calc.odd(2), false)
#test(calc.odd(-1), true)
#test(calc.even(-11), false)

--- calc-rem ---
// Test the `rem` function.
#test(calc.rem(1, 1), 0)
#test(calc.rem(5, 3), 2)
#test(calc.rem(5, -3), 2)
#test(calc.rem(22.5, 10), 2.5)
#test(calc.rem(9, 4.5), 0)
#test(calc.rem(decimal("5"), -3), decimal("2"))
#test(calc.rem(decimal("22.5"), decimal("10")), decimal("2.5"))
#test(calc.rem(9, decimal("4.5")), decimal("0"))
#test(calc.rem(decimal("7"), decimal("3")), decimal("1"))
#test(calc.rem(decimal("7"), decimal("-3")), decimal("1"))
#test(calc.rem(decimal("-7"), decimal("3")), decimal("-1"))
#test(calc.rem(decimal("-7"), decimal("-3")), decimal("-1"))

--- calc-rem-divisor-zero-1 ---
// Error: 14-15 divisor must not be zero
#calc.rem(5, 0)

--- calc-rem-divisor-zero-2 ---
// Error: 16-19 divisor must not be zero
#calc.rem(3.0, 0.0)

--- calc-rem-divisor-zero-3 ---
// Error: 27-39 divisor must not be zero
#calc.rem(decimal("4.0"), decimal("0"))

--- calc-div-euclid ---
// Test the `div-euclid` function.
#test(calc.div-euclid(7, 3), 2)
#test(calc.div-euclid(7, -3), -2)
#test(calc.div-euclid(-7, 3), -3)
#test(calc.div-euclid(-7, -3), 3)
#test(calc.div-euclid(2.5, 2), 1)
#test(calc.div-euclid(decimal("7"), decimal("3")), decimal("2"))
#test(calc.div-euclid(decimal("7"), decimal("-3")), decimal("-2"))
#test(calc.div-euclid(decimal("-7"), decimal("3")), decimal("-3"))
#test(calc.div-euclid(decimal("-7"), decimal("-3")), decimal("3"))
#test(calc.div-euclid(decimal("2.5"), decimal("2")), decimal("1"))

--- calc-div-euclid-divisor-zero-1 ---
// Error: 21-22 divisor must not be zero
#calc.div-euclid(5, 0)

--- calc-div-euclid-divisor-zero-2 ---
// Error: 23-26 divisor must not be zero
#calc.div-euclid(3.0, 0.0)

--- calc-div-euclid-divisor-zero-3 ---
// Error: 35-50 divisor must not be zero
#calc.div-euclid(decimal("3.00"), decimal("0.00"))

--- calc-rem-euclid ---
// Test the `rem-euclid` function.
#test(calc.rem-euclid(7, 3), 1)
#test(calc.rem-euclid(7, -3), 1)
#test(calc.rem-euclid(-7, 3), 2)
#test(calc.rem-euclid(-7, -3), 2)
#test(calc.rem-euclid(2.5, 2), 0.5)
#test(calc.rem-euclid(decimal("7"), decimal("3")), decimal("1"))
#test(calc.rem-euclid(decimal("7"), decimal("-3")), decimal("1"))
#test(calc.rem-euclid(decimal("-7"), decimal("3")), decimal("2"))
#test(calc.rem-euclid(decimal("-7"), decimal("-3")), decimal("2"))
#test(calc.rem-euclid(decimal("2.5"), decimal("2")), decimal("0.5"))

--- calc-rem-euclid-divisor-zero-1 ---
// Error: 21-22 divisor must not be zero
#calc.rem-euclid(5, 0)

--- calc-rem-euclid-divisor-zero-2 ---
// Error: 23-26 divisor must not be zero
#calc.rem-euclid(3.0, 0.0)

--- calc-rem-euclid-divisor-zero-3 ---
// Error: 35-50 divisor must not be zero
#calc.rem-euclid(decimal("3.00"), decimal("0.00"))

--- calc-quo ---
// Test the `quo` function.
#test(calc.quo(1, 1), 1)
#test(calc.quo(5, 3), 1)
#test(calc.quo(5, -3), -1)
#test(calc.quo(22.5, 10), 2.0)
#test(calc.quo(9, 4.5), 2.0)
#test(calc.quo(decimal("22.5"), 10), decimal("2"))
#test(calc.quo(decimal("9"), decimal("4.5")), decimal("2"))

--- calc-quo-divisor-zero-1 ---
// Error: 14-15 divisor must not be zero
#calc.quo(5, 0)

--- calc-quo-divisor-zero-2 ---
// Error: 16-19 divisor must not be zero
#calc.quo(3.0, 0.0)

--- calc-quo-divisor-zero-3 ---
// Error: 27-41 divisor must not be zero
#calc.quo(decimal("4.0"), decimal("0.0"))

--- calc-min-and-max ---
// Test the `min` and `max` functions.
#test(calc.min(2, -4), -4)
#test(calc.min(3.5, 1e2, -0.1, 3), -0.1)
#test(calc.min(decimal("3.5"), 4, decimal("-3213.99999")), decimal("-3213.99999"))
#test(calc.max(-3, 11), 11)
#test(calc.max(decimal("3"), 45), 45)
#test(calc.min("hi"), "hi")

--- calc-pow-log-exp-ln ---
// Test the `pow`, `log`, `exp`, and `ln` functions.
#test(calc.pow(10, 0), 1)
#test(calc.pow(2, 4), 16)
#test(calc.pow(decimal("0.5"), 18), decimal("0.000003814697265625"))
#test(calc.exp(2), calc.pow(calc.e, 2))
#test(calc.ln(10), calc.log(10, base: calc.e))

--- calc-bit-logical ---
// Test the `bit-not`, `bit-and`, `bit-or` and `bit-xor` functions.
#test(64.bit-not(), -65)
#test(0.bit-not(), -1)
#test((-56).bit-not(), 55)
#test(128.bit-and(192), 128)
#test(192.bit-and(224), 192)
#test((-1).bit-and(325532), 325532)
#test(0.bit-and(-53), 0)
#test(0.bit-or(-1), -1)
#test(5.bit-or(3), 7)
#test((-50).bit-or(3), -49)
#test(64.bit-or(32), 96)
#test((-1).bit-xor(1), -2)
#test(64.bit-xor(96), 32)
#test((-1).bit-xor(-7), 6)
#test(0.bit-xor(492), 492)

--- calc-bit-shift ---
// Test the `bit-lshift` and `bit-rshift` functions.
#test(32.bit-lshift(2), 128)
#test(694.bit-lshift(0), 694)
#test(128.bit-rshift(2), 32)
#test(128.bit-rshift(12345), 0)
#test((-7).bit-rshift(2), -2)
#test((-7).bit-rshift(12345), -1)
#test(128.bit-rshift(2, logical: true), 32)
#test((-7).bit-rshift(61, logical: true), 7)
#test(128.bit-rshift(12345, logical: true), 0)
#test((-7).bit-rshift(12345, logical: true), 0)

--- calc-bit-shift-too-large ---
// Error: 2-18 the result is too large
#1.bit-lshift(64)

--- calc-bit-lshift-negative ---
// Error: 15-17 number must be at least zero
#1.bit-lshift(-1)

--- calc-bit-rshift-negative ---
// Error: 15-17 number must be at least zero
#1.bit-rshift(-1)

--- calc-pow-zero-to-power-of-zero ---
// Error: 2-16 zero to the power of zero is undefined
#calc.pow(0, 0)

--- calc-pow-exponent-too-large ---
// Error: 14-31 exponent is too large
#calc.pow(2, 10000000000000000)

--- calc-pow-too-large ---
// Error: 2-25 the result is too large
#calc.pow(2, 2147483647)

--- calc-pow-too-large-decimal ---
// Error: 2-56 the result is too large
#calc.pow(decimal("2222222222222222222222222222"), 100)

--- calc-pow-bad-exponent ---
// Error: 14-36 exponent may not be infinite, subnormal, or NaN
#calc.pow(2, calc.pow(2.0, 10000.0))

--- calc-pow-not-real ---
// Error: 2-19 the result is not a real number
#calc.pow(-1, 0.5)

--- calc-sqrt-not-real ---
// Error: 12-14 cannot take square root of negative number
#calc.sqrt(-1)

--- calc-root ---
#test(calc.root(12.0, 1), 12.0)
#test(calc.root(9.0, 2), 3.0)
#test(calc.root(27.0, 3), 3.0)
#test(calc.root(-27.0, 3), -3.0)
// 100^(-1/2) = (100^(1/2))^-1 = 1/sqrt(100)
#test(calc.root(100.0, -2), 0.1)

--- calc-root-zeroth ---
// Error: 17-18 cannot take the 0th root of a number
#calc.root(1.0, 0)

--- calc-root-negative-even ---
// Error: 24-25 negative numbers do not have a real nth root when n is even
#test(calc.root(-27.0, 4), -3.0)

--- calc-log-negative ---
// Error: 11-13 value must be strictly positive
#calc.log(-1)

--- calc-log-bad-base ---
// Error: 20-21 base may not be zero, NaN, infinite, or subnormal
#calc.log(1, base: 0)

--- calc-log-not-real ---
// Error: 2-24 the result is not a real number
#calc.log(10, base: -1)

--- calc-fact ---
// Test the `fact` function.
#test(calc.fact(0), 1)
#test(calc.fact(5), 120)

--- calc-fact-too-large ---
// Error: 2-15 the result is too large
#calc.fact(21)

--- calc-perm ---
// Test the `perm` function.
#test(calc.perm(0, 0), 1)
#test(calc.perm(5, 3), 60)
#test(calc.perm(5, 5), 120)
#test(calc.perm(5, 6), 0)

--- calc-perm-too-large ---
// Error: 2-19 the result is too large
#calc.perm(21, 21)

--- calc-binom ---
// Test the `binom` function.
#test(calc.binom(0, 0), 1)
#test(calc.binom(5, 3), 10)
#test(calc.binom(5, 5), 1)
#test(calc.binom(5, 6), 0)
#test(calc.binom(6, 2), 15)

--- calc-gcd ---
// Test the `gcd` function.
#test(calc.gcd(112, 77), 7)
#test(calc.gcd(12, 96), 12)
#test(calc.gcd(13, 9), 1)
#test(calc.gcd(13, -9), 1)
#test(calc.gcd(272557, 272557), 272557)
#test(calc.gcd(0, 0), 0)
#test(calc.gcd(7, 0), 7)

--- calc-lcm ---
// Test the `lcm` function.
#test(calc.lcm(112, 77), 1232)
#test(calc.lcm(12, 96), 96)
#test(calc.lcm(13, 9), 117)
#test(calc.lcm(13, -9), 117)
#test(calc.lcm(272557, 272557), 272557)
#test(calc.lcm(0, 0), 0)
#test(calc.lcm(8, 0), 0)

--- calc-lcm-too-large ---
// Error: 2-41 the result is too large
#calc.lcm(15486487489457, 4874879896543)

--- calc-rounding-larger-than-max-int ---
#test(calc.round(decimal("9223372036854775809.5")), decimal("9223372036854775810"))
#test(calc.round(9223372036854775809.5), 9223372036854775810.0)
#test(calc.floor(decimal("9223372036854775809.5")), decimal("9223372036854775809"))
#test(calc.floor(9223372036854775809.5), 9223372036854775809.0)
#test(calc.ceil(decimal("9223372036854775809.5")), decimal("9223372036854775810"))
#test(calc.ceil(9223372036854775809.5), 9223372036854775810.0)
#test(calc.trunc(decimal("9223372036854775809.5")), decimal("9223372036854775809"))
#test(calc.trunc(9223372036854775809.5), 9223372036854775809.0)
#test(calc.quo(decimal("9223372036854775809.5"), 1), decimal("9223372036854775809"))
#test(calc.quo(9223372036854775809.5, 1), 9223372036854775809.0)

--- calc-rounding-smaller-than-min-int ---
#test(calc.round(decimal("-9223372036854775809.5")), decimal("-9223372036854775810"))
#test(calc.round(-9223372036854775809.5), -9223372036854775810.0)
#test(calc.floor(decimal("-9223372036854775809.5")), decimal("-9223372036854775810"))
#test(calc.floor(-9223372036854775809.5), -9223372036854775810.0)
#test(calc.ceil(decimal("-9223372036854775809.5")), decimal("-9223372036854775809"))
#test(calc.ceil(-9223372036854775809.5), -9223372036854775809.0)
#test(calc.trunc(decimal("-9223372036854775809.5")), decimal("-9223372036854775809"))
#test(calc.trunc(-9223372036854775809.5), -9223372036854775809.0)
#test(calc.quo(decimal("-9223372036854775809.5"), 1), decimal("-9223372036854775810"))
#test(calc.quo(-9223372036854775809.5, 1), -9223372036854775810.0)

--- calc-min-nothing ---
// Error: 2-12 expected at least one value
#calc.min()

--- calc-min-uncomparable ---
// Error: 14-18 cannot compare string and integer
#calc.min(1, "hi")

--- calc-max-uncomparable ---
// Error: 16-19 cannot compare 1pt with 1em
#calc.max(1em, 1pt)

--- calc-clamp-decimal-float ---
// Error: 2-37 cannot apply this operation to a decimal and a float
// Hint: 2-37 if loss of precision is acceptable, explicitly cast the decimal to a float with `float(value)`
#calc.clamp(decimal("10"), 5.5, 6.6)

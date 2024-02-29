// Test math functions.
// Ref: false

---
// Test conversion to numbers.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("150"), 150)
#test(int("-834"), -834)
#test(int("\u{2212}79"), -79)
#test(int(10 / 3), 3)
#test(float(10), 10.0)
#test(float(50% * 30%), 0.15)
#test(float("31.4e-1"), 3.14)
#test(float("31.4e\u{2212}1"), 3.14)
#test(float("3.1415"), 3.1415)
#test(float("-7654.321"), -7654.321)
#test(float("\u{2212}7654.321"), -7654.321)
#test(type(float(10)), float)

---
// Test float `is-nan()`.
#test(float(calc.nan).is-nan(), true)
#test(float(10).is-nan(), false)

---
// Test float `is-infinite()`.
#test(float(calc.inf).is-infinite(), true)
#test(float(-calc.inf).is-infinite(), true)
#test(float(10).is-infinite(), false)
#test(float(-10).is-infinite(), false)

---
// Test float `signum()`
#test(float(0.0).signum(), 1.0)
#test(float(1.0).signum(), 1.0)
#test(float(-1.0).signum(), -1.0)
#test(float(10.0).signum(), 1.0)
#test(float(-10.0).signum(), -1.0)
#test(float(calc.nan).signum().is-nan(), true)

---
// Test int `signum()`
#test(int(0).signum(), 0)
#test(int(1.0).signum(), 1)
#test(int(-1.0).signum(), -1)
#test(int(10.0).signum(), 1)
#test(int(-10.0).signum(), -1)

---
#test(calc.round(calc.e, digits: 2), 2.72)
#test(calc.round(calc.pi, digits: 2), 3.14)

---
// Error: 6-10 expected integer, boolean, float, or string, found length
#int(10pt)

---
// Error: 8-13 expected float, boolean, integer, ratio, or string, found type
#float(float)

---
// Error: 6-12 invalid integer: nope
#int("nope")

---
// Error: 8-15 invalid float: 1.2.3
#float("1.2.3")

---
// Test the `abs` function.
#test(calc.abs(-3), 3)
#test(calc.abs(3), 3)
#test(calc.abs(-0.0), 0.0)
#test(calc.abs(0.0), -0.0)
#test(calc.abs(-3.14), 3.14)
#test(calc.abs(50%), 50%)
#test(calc.abs(-25%), 25%)

---
// Error: 11-22 expected integer, float, length, angle, ratio, or fraction, found string
#calc.abs("no number")

---
// Test the `even` and `odd` functions.
#test(calc.even(2), true)
#test(calc.odd(2), false)
#test(calc.odd(-1), true)
#test(calc.even(-11), false)

---
// Test the `rem` function.
#test(calc.rem(1, 1), 0)
#test(calc.rem(5, 3), 2)
#test(calc.rem(5, -3), 2)
#test(calc.rem(22.5, 10), 2.5)
#test(calc.rem(9, 4.5), 0)

---
// Error: 14-15 divisor must not be zero
#calc.rem(5, 0)

---
// Error: 16-19 divisor must not be zero
#calc.rem(3.0, 0.0)

---
// Test the `div-euclid` function.
#test(calc.div-euclid(7, 3), 2)
#test(calc.div-euclid(7, -3), -2)
#test(calc.div-euclid(-7, 3), -3)
#test(calc.div-euclid(-7, -3), 3)
#test(calc.div-euclid(2.5, 2), 1)

---
// Error: 21-22 divisor must not be zero
#calc.div-euclid(5, 0)

---
// Error: 23-26 divisor must not be zero
#calc.div-euclid(3.0, 0.0)

---
// Test the `rem-euclid` function.
#test(calc.rem-euclid(7, 3), 1)
#test(calc.rem-euclid(7, -3), 1)
#test(calc.rem-euclid(-7, 3), 2)
#test(calc.rem-euclid(-7, -3), 2)
#test(calc.rem-euclid(2.5, 2), 0.5)

---
// Error: 21-22 divisor must not be zero
#calc.rem-euclid(5, 0)

---
// Error: 23-26 divisor must not be zero
#calc.rem-euclid(3.0, 0.0)

---
// Test the `quo` function.
#test(calc.quo(1, 1), 1)
#test(calc.quo(5, 3), 1)
#test(calc.quo(5, -3), -1)
#test(calc.quo(22.5, 10), 2)
#test(calc.quo(9, 4.5), 2)

---
// Error: 14-15 divisor must not be zero
#calc.quo(5, 0)

---
// Error: 16-19 divisor must not be zero
#calc.quo(3.0, 0.0)

---
// Test the `min` and `max` functions.
#test(calc.min(2, -4), -4)
#test(calc.min(3.5, 1e2, -0.1, 3), -0.1)
#test(calc.max(-3, 11), 11)
#test(calc.min("hi"), "hi")

---
// Test the `pow`, `log`, `exp`, and `ln` functions.
#test(calc.pow(10, 0), 1)
#test(calc.pow(2, 4), 16)
#test(calc.exp(2), calc.pow(calc.e, 2))
#test(calc.ln(10), calc.log(10, base: calc.e))

---
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

---
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

---
// Error: 2-18 the result is too large
#1.bit-lshift(64)

---
// Error: 15-17 number must be at least zero
#1.bit-lshift(-1)

---
// Error: 15-17 number must be at least zero
#1.bit-rshift(-1)

---
// Error: 2-16 zero to the power of zero is undefined
#calc.pow(0, 0)

---
// Error: 14-31 exponent is too large
#calc.pow(2, 10000000000000000)

---
// Error: 2-25 the result is too large
#calc.pow(2, 2147483647)

---
// Error: 14-36 exponent may not be infinite, subnormal, or NaN
#calc.pow(2, calc.pow(2.0, 10000.0))

---
// Error: 2-19 the result is not a real number
#calc.pow(-1, 0.5)

---
// Error: 12-14 cannot take square root of negative number
#calc.sqrt(-1)

---
#test(calc.root(12.0, 1), 12.0)
#test(calc.root(9.0, 2), 3.0)
#test(calc.root(27.0, 3), 3.0)
#test(calc.root(-27.0, 3), -3.0)
// 100^(-1/2) = (100^(1/2))^-1 = 1/sqrt(100)
#test(calc.root(100.0, -2), 0.1)

---
// Error: 17-18 cannot take the 0th root of a number
#calc.root(1.0, 0)

---
// Error: 24-25 negative numbers do not have a real nth root when n is even
#test(calc.root(-27.0, 4), -3.0)

---
// Error: 11-13 value must be strictly positive
#calc.log(-1)

---
// Error: 20-21 base may not be zero, NaN, infinite, or subnormal
#calc.log(1, base: 0)

---
// Error: 2-24 the result is not a real number
#calc.log(10, base: -1)

---
// Test the `fact` function.
#test(calc.fact(0), 1)
#test(calc.fact(5), 120)

---
// Error: 2-15 the result is too large
#calc.fact(21)

---
// Test the `perm` function.
#test(calc.perm(0, 0), 1)
#test(calc.perm(5, 3), 60)
#test(calc.perm(5, 5), 120)
#test(calc.perm(5, 6), 0)

---
// Error: 2-19 the result is too large
#calc.perm(21, 21)

---
// Test the `binom` function.
#test(calc.binom(0, 0), 1)
#test(calc.binom(5, 3), 10)
#test(calc.binom(5, 5), 1)
#test(calc.binom(5, 6), 0)
#test(calc.binom(6, 2), 15)

---
// Test the `gcd` function.
#test(calc.gcd(112, 77), 7)
#test(calc.gcd(12, 96), 12)
#test(calc.gcd(13, 9), 1)
#test(calc.gcd(13, -9), 1)
#test(calc.gcd(272557, 272557), 272557)
#test(calc.gcd(0, 0), 0)
#test(calc.gcd(7, 0), 7)

---
// Test the `lcm` function.
#test(calc.lcm(112, 77), 1232)
#test(calc.lcm(12, 96), 96)
#test(calc.lcm(13, 9), 117)
#test(calc.lcm(13, -9), 117)
#test(calc.lcm(272557, 272557), 272557)
#test(calc.lcm(0, 0), 0)
#test(calc.lcm(8, 0), 0)

---
// Error: 2-41 the result is too large
#calc.lcm(15486487489457, 4874879896543)

---
// Error: 2-12 expected at least one value
#calc.min()

---
// Error: 14-18 cannot compare string and integer
#calc.min(1, "hi")

---
// Error: 16-19 cannot compare 1pt with 1em
#calc.max(1em, 1pt)

---
// Test the `range` function.
#test(range(4), (0, 1, 2, 3))
#test(range(1, 4), (1, 2, 3))
#test(range(-4, 2), (-4, -3, -2, -1, 0, 1))
#test(range(10, 5), ())
#test(range(10, step: 3), (0, 3, 6, 9))
#test(range(1, 4, step: 1), (1, 2, 3))
#test(range(1, 8, step: 2), (1, 3, 5, 7))
#test(range(5, 2, step: -1), (5, 4, 3))
#test(range(10, 0, step: -3), (10, 7, 4, 1))

---
// Error: 2-9 missing argument: end
#range()

---
// Error: 11-14 expected integer, found float
#range(1, 2.0)

---
// Error: 17-22 expected integer, found string
#range(4, step: "one")

---
// Error: 18-19 number must not be zero
#range(10, step: 0)

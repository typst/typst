// Test math functions.
// Ref: false

---
// Test conversion to numbers.
#test(int(false), 0)
#test(int(true), 1)
#test(int(10), 10)
#test(int("150"), 150)
#test(int(10 / 3), 3)
#test(float(10), 10.0)
#test(float(50% * 30%), 0.15)
#test(float("31.4e-1"), 3.14)
#test(type(float(10)), "float")

---
#test(calc.round(calc.e, digits: 2), 2.72)
#test(calc.round(calc.pi, digits: 2), 3.14)

---
// Error: 6-10 expected boolean, float, string, or integer, found length
#int(10pt)

---
// Error: 8-13 expected boolean, integer, ratio, string, or float, found function
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
// Error: 10-16 zero to the power of zero is undefined
#calc.pow(0, 0)

---
// Error: 14-31 exponent is too large
#calc.pow(2, 10000000000000000)

---
// Error: 10-25 the result is too large
#calc.pow(2, 2147483647)

---
// Error: 14-36 exponent may not be infinite, subnormal, or NaN
#calc.pow(2, calc.pow(2.0, 10000.0))

---
// Error: 10-19 the result is not a real number
#calc.pow(-1, 0.5)

---
// Error: 12-14 cannot take square root of negative number
#calc.sqrt(-1)

---
// Error: 11-13 value must be strictly positive
#calc.log(-1)

---
// Error: 20-21 base may not be zero, NaN, infinite, or subnormal
#calc.log(1, base: 0)

---
// Error: 10-24 the result is not a real number
#calc.log(10, base: -1)

---
// Test the `fact` function.
#test(calc.fact(0), 1)
#test(calc.fact(5), 120)

---
// Error: 11-15 the result is too large
#calc.fact(21)

---
// Test the `perm` function.
#test(calc.perm(0, 0), 1)
#test(calc.perm(5, 3), 60)
#test(calc.perm(5, 5), 120)
#test(calc.perm(5, 6), 0)

---
// Error: 11-19 the result is too large
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
// Error: 10-41 the return value is too large
#calc.lcm(15486487489457, 4874879896543)

---
// Error: 10-12 expected at least one value
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
// Error: 7-9 missing argument: end
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

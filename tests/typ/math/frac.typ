// Test fractions.

---
// Test that denominator baseline matches in the common case.
$ x = 1/2 = a/(a h) = a/a = a/(1/2) $

---
// Test parenthesis removal.
$ (|x| + |y|)/2 < [1+2]/3 $

---
// Test large fraction.
$ x = (-b plus.minus sqrt(b^2 - 4a c))/(2a) $

---
// Test binomial.
$ binom(circle, square) $

---
// Test multinomial coefficients.
$ binom(n, k_1, k_2, k_3) $

---
// Error: 8-13 missing argument: lower
$ binom(x^2) $

---
// Test dif.
$ (dif y)/(dif x), dif/x, x/dif, dif/dif \
  frac(dif y, dif x), frac(dif, x), frac(x, dif), frac(dif, dif) $

---
// Test associativity.
$ 1/2/3 = (1/2)/3 = 1/(2/3) $

---
// Test precedence.
$ a_1/b_2, 1/f(x), zeta(x)/2, "foo"[|x|]/2 \
  1.2/3.7, 2.3^3.4 \
  🏳️‍🌈[x]/2, f [x]/2, phi [x]/2, 🏳️‍🌈 [x]/2 \
  +[x]/2, 1(x)/2, 2[x]/2 \
  (a)b/2, b(a)[b]/2 \
  n!/2, 5!/2, n !/2, 1/n!, 1/5! $

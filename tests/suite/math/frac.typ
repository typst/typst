// Test fractions.

--- math-frac-baseline ---
// Test that denominator baseline matches in the common case.
$ x = 1/2 = a/(a h) = a/a = a/(1/2) $

--- math-frac-paren-removal ---
// Test parenthesis removal.
$ (|x| + |y|)/2 < [1+2]/3 $

--- math-frac-large ---
// Test large fraction.
$ x = (-b plus.minus sqrt(b^2 - 4a c))/(2a) $

--- math-binom ---
// Test binomial.
$ binom(circle, square) $

--- math-binom-multiple ---
// Test multinomial coefficients.
$ binom(n, k_1, k_2, k_3) $

--- math-binom-missing-lower ---
// Error: 3-13 missing argument: lower
$ binom(x^2) $

--- math-dif ---
// Test dif.
$ (dif y)/(dif x), dif/x, x/dif, dif/dif \
  frac(dif y, dif x), frac(dif, x), frac(x, dif), frac(dif, dif) $

--- math-frac-associativity ---
// Test associativity.
$ 1/2/3 = (1/2)/3 = 1/(2/3) $

--- math-frac-precedence ---
// Test precedence.
$ a_1/b_2, 1/f(x), zeta(x)/2, "foo"[|x|]/2 \
  1.2/3.7, 2.3^3.4 \
  f [x]/2, phi [x]/2 \
  +[x]/2, 1(x)/2, 2[x]/2, 🏳️‍🌈[x]/2 \
  (a)b/2, b(a)[b]/2 \
  n!/2, 5!/2, n !/2, 1/n!, 1/5! $

--- math-frac-gap ---
// Test that the gap above and below the fraction rule is correct.
$ sqrt(n^(2/3)) $

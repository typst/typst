// Test fractions.

--- math-frac-baseline paged ---
// Test that denominator baseline matches in the common case.
$ x = 1/2 = a/(a h) = a/a = a/(1/2) $

--- math-frac-paren-removal paged ---
// Test parenthesis removal.
$ (|x| + |y|)/2 < [1+2]/3 $

--- math-frac-large paged ---
// Test large fraction.
$ x = (-b plus.minus sqrt(b^2 - 4a c))/(2a) $

--- math-binom paged ---
// Test binomial.
$ binom(circle, square) $

--- math-binom-multiple paged ---
// Test multinomial coefficients.
$ binom(n, k_1, k_2, k_3) $

--- math-binom-missing-lower paged ---
// Error: 3-13 missing argument: lower
$ binom(x^2) $

--- math-dif paged ---
// Test dif.
$
  (dif y)/(dif x), dif/x, x/dif, dif/dif \
  frac(dif y, dif x), frac(dif, x), frac(x, dif), frac(dif, dif)
$

--- math-frac-associativity paged ---
// Test associativity.
$ 1/2/3 = (1/2)/3 = 1/(2/3) $

--- math-frac-tan-sin-cos paged ---
// A nice simple example of a simple trig property.
$
  tan(x) = sin(x) / cos(x) \
  tan x = (sin x) / (cos x)
$

--- math-frac-precedence paged ---
// Test precedence.
$
  a_1/b_2, 1/f(x), zeta(x)/2, "foo"[|x|]/2 \
  1.2/3.7, 2.3^3.4 \
  f [x]/2, phi [x]/2 \
  +[x]/2, 1(x)/2, 2[x]/2, 🏳️‍🌈[x]/2 \
  (a)b/2, b(a)[b]/2 \
  n!/2, 5!/2, n !/2, 1/n!, 1/5!
$

--- math-frac-implicit-func paged ---
// Test other precedence interactions with implicit function calls.
$
  f'(x) / f_pi{x} \
  sin^2(x) / f_0(x) quad f!(x) / g^(-1)(x) \
  a_\u{2a}[|x} / a_"2a"{x|] quad f_pi.alt{x} / f_#math.pi.alt{x} \
  a(b)_c(d)^e(f) / g(h)'_i(j)' \
  (x)'(x)'(x)' / (x)'(x)'(x)' \
$

--- math-frac-gap paged ---
// Test that the gap above and below the fraction rule is correct.
$ sqrt(n^(2/3)) $

--- math-frac-horizontal paged ---
// Test that horizontal fractions look identical to inline math with `slash`
#set math.frac(style: "horizontal")
$ (a / b) / (c / (d / e)) $
$ (a slash b) slash (c slash (d slash e)) $

--- math-frac-horizontal-lr-paren paged ---
// Test that parentheses are in a left-right pair even when rebuilt by a horizontal fraction
#set math.frac(style: "horizontal")
$ (#v(2em)) / n $

--- math-frac-skewed paged ---
// Test skewed fractions
#set math.frac(style: "skewed")
$ a / b, a / (b / c) $

--- math-frac-horizontal-explicit paged ---
// Test that explicit fractions don't change parentheses
#set math.frac(style: "horizontal")
$ frac(a, (b + c)), frac(a, b + c) $

--- math-frac-horizontal-nonparen-brackets paged ---
// Test that non-parentheses left-right pairs remain untouched
#set math.frac(style: "horizontal")
$ [x+y] / {z} $

--- math-frac-styles-inline paged ---
// Test inline layout of styled fractions
#set math.frac(style: "horizontal")
$a/(b+c), frac(a, b+c, style: "skewed"), frac(a, b+c, style: "vertical")$

--- math-frac-text-decoration paged ---
#set text(size: 20pt)
#text(fill: red)[$a = F / m$] \
#text(stroke: red + .5pt)[$a = F/m$]

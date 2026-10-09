// Test fractions.

--- math-frac-baseline paged html ---
// Test that denominator baseline matches in the common case.
$ x = 1/2 = a/(a h) = a/a = a/(1/2) $

--- math-frac-paren-removal paged html ---
// Test parenthesis removal.
$ (|x| + |y|)/2 < [1+2]/3 $

--- math-frac-large paged html ---
// Test large fraction.
$ x = (-b plus.minus sqrt(b^2 - 4a c))/(2a) $

--- math-binom paged html ---
// Test binomial.
$ binom(circle, square) $

--- math-binom-multiple paged html ---
// Test multinomial coefficients.
$ binom(n, k_1, k_2, k_3) $

--- math-binom-missing-lower eval ---
// Error: 3-13 missing argument: lower
$ binom(x^2) $

--- math-dif paged html ---
// Test dif.
$ (dif y)/(dif x), dif/x, x/dif, dif/dif \
  frac(dif y, dif x), frac(dif, x), frac(x, dif), frac(dif, dif) $

--- math-frac-associativity paged html ---
// Test associativity.
$ 1/2/3 = (1/2)/3 = 1/(2/3) $

--- math-frac-tan-sin-cos paged html ---
// A nice simple example of a simple trig property.
$ tan(x) = sin(x) / cos(x) \
  tan x = (sin x) / (cos x) $

--- math-frac-precedence paged html ---
// Test precedence.
$ a_1/b_2, 1/f(x), zeta(x)/2, "foo"[|x|]/2 \
  1.2/3.7, 2.3^3.4 \
  f [x]/2, phi [x]/2 \
  +[x]/2, 1(x)/2, 2[x]/2, 🏳️‍🌈[x]/2 \
  (a)b/2, b(a)[b]/2 \
  n!/2, 5!/2, n !/2, 1/n!, 1/5! $

--- math-frac-implicit-func paged html ---
// Test other precedence interactions with implicit function calls.
$
  f'(x) / f_pi{x} \
  sin^2(x) / f_0(x) quad f!(x) / g^(-1)(x) \
  a_\u{2a}[|x} / a_"2a"{x|] quad f_pi.alt{x} / f_#math.pi.alt{x} \
  a(b)_c(d)^e(f) / g(h)'_i(j)' \
  (x)'(x)'(x)' / (x)'(x)'(x)' \
$

--- math-frac-precedence-xid-vs-alpha eval ---
// Many characters have the Unicode `Alphabetic` property but not `xid_start`,
// but only two have `xid_start` but not `Alphabetic` (℮ U+212E and ℘ U+2118).
// We used to use `Alphabetic` to determine whether characters would be treated
// as an implicit function call, but now we use `xid_start`. This is partly
// because the chars with `Alphabetic` but not `xid_start` are not very
// function-like, but mainly because ℘ is an actual named function!
// https://en.wikipedia.org/wiki/Weierstrass_elliptic_function
// https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=\p{alpha}+-+\p{xids}

#let alphabetic = regex("\p{alpha}")
#let xid-start = regex("\p{xids}")

// https://en.wikipedia.org/wiki/Weierstrass_elliptic_function
#assert(not "℘".contains(alphabetic))
#assert("℘".contains(xid-start))
#let eqn = $ ℘()/2  $
#test(repr(eqn.body), "frac(
  num: sequence([℘], lr(body: sequence([(], [)]))),
  denom: [2],
)")

#assert("ⓟ".contains(alphabetic))
#assert(not "ⓟ".contains(xid-start))
#let eqn = $ ⓟ()/2  $
#test(repr(eqn.body), "sequence([ⓟ], frac(num: [], denom: [2]))")

--- math-frac-gap paged html ---
// Test that the gap above and below the fraction rule is correct.
$ sqrt(n^(2/3)) $

--- math-frac-horizontal paged html ---
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
$ a / b,  a / (b / c) $

--- math-frac-horizontal-explicit paged html ---
// Test that explicit fractions don't change parentheses
#set math.frac(style: "horizontal")
$ frac(a, (b + c)), frac(a, b + c) $

--- math-frac-horizontal-nonparen-brackets paged html ---
// Test that non-parentheses left-right pairs remain untouched
#set math.frac(style: "horizontal")
$ [x+y] / {z} $

--- math-frac-styles-inline paged ---
// Test inline layout of styled fractions
#set math.frac(style: "horizontal")
$a/(b+c), frac(a, b+c, style: "skewed"), frac(a, b+c, style: "vertical")$

--- math-frac-line-fill-stroke paged html ---
// Test that the horizontal stroke is also decorated like text glyphs
#text(size: 20pt, fill: yellow, stroke: red + .5pt)[$1/Delta$]
#text(size: 25pt, stroke: red)[$1/Delta$]

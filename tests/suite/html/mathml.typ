--- mathml-dif html ---
$ integral dif x Dif y $

--- mathml-vary-class-spacing html ---
$ x = - 1 quad x - 1 $

--- mathml-text-op html ---
$ tan x = (sin x)/(cos x) $
$ op("custom", limits: #true)_(n -> oo) n $

--- mathml-lr html ---
$ (x/2 + z) $

--- mathml-lr-size html ---
$ lr((a + b), size: #5em) $

--- mathml-spacing-resolve html ---
$
  a #h(1em + 10pt) b \
  c #h(2em) d \
  e #h(1em) #h(10pt, weak: true) e
$

--- mathml-equation-alignment-spacing html ---
$
    a & = b + c \
  d + & - f
$

--- mathml-equation-alignment html ---
$
  a & = b + c \
    & = d + e \
    & = f + g
$

--- mathml-stretch-largeop-vs-explicit html ---
$ integral $
$ stretch(integral) $
$ stretch(integral, size: #50pt) $
$ stretch(integral, size: #50%) $

--- mathml-stretch html ---
$ a -> b $
$ a stretch(->) b $
$ a stretch(->, size: #3em) b $
$ a stretch(->, size: #200%) b $

--- mathml-custom-class html ---
#let loves = math.class(
  "relation",
  sym.suit.heart,
)
$x loves y and y loves 5$

--- mathml-primes html ---
$ x' x'' x''' x'''' x''''' x'''''' $

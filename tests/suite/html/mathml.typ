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

--- mathml-stretch-vertical-alignment html ---
$ stretch(\[, size: #4em) a/b stretch(arrow.t, size: #4em) $

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

--- mathml-primes-factorial-explicit-class html ---
$ n! + (n+1)! + class("punctuation", !) + class("opening", !) + ! $
$ n' + (n+1)' + class("punctuation", ') + class("opening", ') + ' $

--- mathml-ignored-math-content html ---
#set math.frac(style: "skewed")
$
  // Warning: 7-16 cancel was ignored during MathML export
  // Warning: 19-30 overline was ignored during MathML export
  // Warning: 19-32 skewed fraction was ignored during MathML export
  // Warning: 35-47 underline was ignored during MathML export
  a + cancel(x) + overline(y)/d = underline(x) - 1
$

--- mathml-ignored-external-content html ---
$
  // Warning: 23-30 grid was ignored during HTML export
  a + #box[b] + c != #grid[d]
$

--- mathml-inline-multiline-equation html ---
Blah $a \ b$ blah.

Blah $a &= b + c \ d + e &= f$ blah.

Blah $a + b + c \ d$ blah.

--- mathml-attach-single-op html ---
$ f^+ $
$ f^times $
$ ZZ^(plus.o r) ZZ^(plus.o.big r) $

--- mathml-box-equation html ---
A #box($ sum_(i = 0)^oo $) B #box($sum_(i = 0)^oo$) C

--- mathml-block-equation html ---
A #block($sum_(i = 0)^oo$) B #block($ sum_(i = 0)^oo $) C

--- mathml-box-block-equation-nested html ---
#block(box($ sum_(i = 0)^oo $)) vs #box(block($sum_(i = 0)^oo$))

--- mathml-html-elem-styles html ---
#show math.attach: set bibliography(title: "Hi")
#show html.elem.where(tag: "mi"): context bibliography.title
$ a^+ $

--- mathml-html-elem-show-rule html ---
#show html.elem.where(tag: "mo"): none
$ a^(1+2) $

--- mathml-show-rule html ---
#show math.frac: it => html.elem("mrow", it)
$ a/b $

--- mathml-show-rule-paged paged empty ---
// Warning: 24-45 MathML element was ignored during paged export
#show math.frac: it => html.elem("mrow", it)
$ a/b $

--- mathml-nested-elem-show-rule html ---
#set text(10pt)
#show math.attach: set text(size: 2em)
#show html.elem.where(tag: "mi"): context repr(text.size)
$ (a)^+ $

--- mathml-non-math-in-math html ---
#html.elem("mtext")[*bold* text]
#html.elem("mtext", html.div())

--- mathml-show-rule-non-math-in-math html ---
#show math.attach: html.elem("mn")[*bold* text]
$ a^1 $

--- mathml-show-rule-non-math-in-math-in-math html ---
#show html.elem.where(tag: "msqrt"): html.elem("mtext")[*bold* text]
$ a/sqrt(b) $

--- mathml-show-rule-math-in-non-math-in-math html ---
#show html.elem.where(tag: "msqrt"): html.elem("div", $x^2$)
$ a/sqrt(b) $

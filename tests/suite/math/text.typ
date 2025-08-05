// Test that setting font features in math.equation has an effect.

--- math-font-fallback ---
// Test font fallback.
$ よ and 🏳️‍🌈 $

--- math-text-color ---
// Test text properties.
$text(#red, "time"^2) + sqrt("place")$

--- math-font-features ---
$ nothing $
$ "hi ∅ hey" $
$ sum_(i in NN) 1 + i $
#show math.equation: set text(features: ("cv01",), fallback: false)
$ nothing $
$ "hi ∅ hey" $
$ sum_(i in NN) 1 + i $

--- math-font-features-switch ---
#let scr(it) = text(features: ("ss01",), $cal(it)$)
$cal(P)_i != scr(P)_i$, $cal(bold(I))_l != bold(scr(I))_l$
$ product.co_(B in scr(B))^(B in scr(bold(B))) cal(B)(X) $

--- math-font-covers ---
#show math.equation: set text(
  font: (
    // Ignore that this regex actually misses some of the script glyphs...
    (name: "XITS Math", covers: regex("[\u{1D49C}-\u{1D503}]")),
    "New Computer Modern Math"
  ),
  features: ("ss01",),
)
$ cal(P)_i (X) * cal(C)_1 $

--- math-font-warning ---
#show math.equation: set text(font: "Libertinus Serif")
// Warning: 1-14 current font is not designed for math
// Hint: 1-14 rendering may be poor
$ x + y = z $

--- math-optical-size-nested-scripts ---
// Test transition from script to scriptscript.
#[
#set text(size:20pt)
$  e^(e^(e^(e))) $
]
A large number: $e^(e^(e^(e)))$.

--- math-optical-size-primes ---
//  Test prime/double prime via scriptsize
#let prime = [ \u{2032} ]
#let dprime = [ \u{2033} ]
#let tprime = [ \u{2034} ]
$ y^dprime-2y^prime + y = 0 $
$y^dprime-2y^prime + y = 0$
$ y^tprime_3 + g^(prime 2) $

--- math-optical-size-prime-large-operator ---
// Test prime superscript on large symbol
$ scripts(sum_(k in NN))^prime 1/k^2 $
$sum_(k in NN)^prime 1/k^2$

--- math-optical-size-frac-script-script ---
// Test script-script in a fraction.
$ 1/(x^A) $
#[#set text(size:18pt); $1/(x^A)$] vs. #[#set text(size:14pt); $x^A$]

--- math-par ---
// Ensure that math does not produce paragraphs.
#show par: highlight
$ a + "bc" + #[c] + #box[d] + #block[e] $

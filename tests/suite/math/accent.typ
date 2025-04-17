// Test math accents.

--- math-accent-sym-call ---
// Test function call.
$grave(a), acute(b), hat(f), tilde(§), macron(ä), diaer(a), ä \
 breve(\&), dot(!), circle(a), caron(@), arrow(Z), arrow.l(Z)$

--- math-accent-align ---
$ x &= p \ dot(x) &= v \ dot.double(x) &= a \ dot.triple(x) &= j \ dot.quad(x) &= s $

--- math-accent-func ---
// Test `accent` function.
$accent(ö, .), accent(v, <-), accent(ZZ, \u{0303})$

--- math-accent-bounds ---
// Test accent bounds.
$sqrt(tilde(T)) + hat(f)/hat(g)$

--- math-accent-wide-base ---
// Test wide base.
$arrow("ABC" + d), tilde(sum)$

--- math-accent-superscript ---
// Test effect of accent on superscript.
$A^x != hat(A)^x != hat(hat(A))^x$

--- math-accent-high-base ---
// Test high base.
$ tilde(integral), tilde(integral)_a^b, tilde(integral_a^b) $

--- math-accent-sized ---
// Test accent size.
$tilde(sum), tilde(sum, size: #50%), accent(H, hat, size: #200%)$

--- math-accent-sized-script ---
// Test accent size in script size.
$tilde(U, size: #1.1em), x^tilde(U, size: #1.1em), sscript(tilde(U, size: #1.1em))$

--- math-accent-dotless ---
// Test dotless glyph variants.
#let test(c) = $grave(#c), acute(sans(#c)), hat(frak(#c)), tilde(mono(#c)),
  macron(bb(#c)), dot(cal(#c)), diaer(upright(#c)), breve(bold(#c)),
  circle(bold(upright(#c))), caron(upright(sans(#c))), arrow(bold(frak(#c)))$
$test(i) \ test(j)$

--- math-accent-dotless-disabled ---
// Test disabling the dotless glyph variants.
$hat(i), hat(i, dotless: #false), accent(j, tilde), accent(j, tilde, dotless: #false)$

--- math-accent-dotless-set-rule ---
#set math.accent(dotless: false)
$ hat(i) $

--- math-accent-flattened ---
// Test flattened accent glyph variants.
#show math.equation: set text(font: "STIX Two Math")
$hat(a) hat(A)$
$tilde(w) tilde(W)$
$grave(i) grave(j)$
$grave(I) grave(J)$

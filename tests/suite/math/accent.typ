// Test math accents.

--- math-accent-sym-call render ---
// Test function call.
$grave(a), acute(b), hat(f), tilde(§), macron(ä), diaer(a), ä \
 breve(\&), dot(!), circle(a), caron(@), arrow(Z), arrow.l(Z)$

--- math-accent-align render ---
$ x &= p \ dot(x) &= v \ dot.double(x) &= a \ dot.triple(x) &= j \ dot.quad(x) &= s $

--- math-accent-func render ---
// Test `accent` function.
$accent(ö, .), accent(v, <-), accent(ZZ, \u{0303})$

--- math-accent-bounds render ---
// Test accent bounds.
$sqrt(tilde(T)) + hat(f)/hat(g)$

--- math-accent-wide-base render ---
// Test wide base.
$arrow("ABC" + d), tilde(sum)$

--- math-accent-superscript render ---
// Test effect of accent on superscript.
$A^x != hat(A)^x != hat(hat(A))^x$

--- math-accent-high-base render ---
// Test high base.
$ tilde(integral), tilde(integral)_a^b, tilde(integral_a^b) $

--- math-accent-sized render ---
// Test accent size.
$tilde(sum), tilde(sum, size: #50%), accent(H, hat, size: #200%)$

--- math-accent-sized-script render ---
// Test accent size in script size.
$tilde(U, size: #1.1em), x^tilde(U, size: #1.1em), sscript(tilde(U, size: #1.1em))$

--- math-accent-dotless render ---
// Test dotless glyph variants.
#let test(c) = $grave(#c), acute(sans(#c)), hat(frak(#c)), tilde(mono(#c)),
  macron(bb(#c)), dot(cal(#c)), diaer(upright(#c)), breve(bold(#c)),
  circle(bold(upright(#c))), caron(upright(sans(#c))), arrow(bold(frak(#c)))$
$test(i) \ test(j)$

--- math-accent-dotless-disabled render ---
// Test disabling the dotless glyph variants.
$hat(i), hat(i, dotless: #false), accent(j, tilde), accent(j, tilde, dotless: #false)$

--- math-accent-dotless-set-rule render ---
#set math.accent(dotless: false)
$ hat(i) $

--- math-accent-dotless-greedy render ---
// Currently the dotless style propagates to everything in the accent's base,
// even though it shouldn't.
$ arrow(P_(c, i dot j) P_(1, i) j) \
  arrow(P_(c, i dot j) P_(1, i) j, dotless: #false) $

--- math-accent-flattened render ---
// Test flattened accent glyph variants.
#show math.equation: set text(font: "STIX Two Math")
$hat(a) hat(A)$
$tilde(w) tilde(W)$
$grave(i) grave(j)$
$grave(I) grave(J)$

--- math-accent-bottom render ---
// Test bottom accents.
$accent(a, \u{20EE}), accent(T, \u{0323}), accent(xi, \u{0332}),
  accent(f, \u{20ED}), accent(F, \u{20E8}), accent(y, \u{032E}),
  accent(!, \u{032F}), accent(J, \u{0333}), accent(p, \u{0331})$

--- math-accent-bottom-wide-base render ---
// Test wide base with bottom accents.
$accent(x + y, \u{20EF}), accent(sum, \u{032D})$

--- math-accent-bottom-subscript render ---
// Test effect of bottom accent on subscript.
$q_x != accent(q, \u{032C})_x != accent(accent(q, \u{032C}), \u{032C})_x$

--- math-accent-bottom-high-base render ---
// Test high base with bottom accents.
$ accent(integral, \u{20EC}), accent(integral, \u{20EC})_a^b, accent(integral_a^b, \u{20EC}) $

--- math-accent-bottom-sized render ---
// Test bottom accent size.
$accent(sum, \u{0330}), accent(sum, \u{0330}, size: #50%), accent(H, \u{032D}, size: #200%)$

--- math-accent-nested render ---
// Test nested top and bottom accents.
$hat(accent(L, \u{0330})), accent(circle(p), \u{0323}),
  macron(accent(caron(accent(A, \u{20ED})), \u{0333})) \
  breve(accent(eta, \u{032E})) = accent(breve(eta), \u{032E})$

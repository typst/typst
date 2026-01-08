// Test math accents.

--- math-accent-sym-call paged ---
// Test function call.
$ grave(a), acute(b), hat(f), tilde(§), macron(ä), dash(ä), breve(ä), \
  dot(!), dot.double(a), diaer(a), dot.triple(a), dot.quad(a), circle(a), \
  acute.double(a), caron(@), arrow(Z), arrow.l(Z), arrow.l.r(Z), \
  harpoon(a), harpoon.lt(a) $

--- math-accent-align paged ---
$ x &= p \ dot(x) &= v \ dot.double(x) &= a \ dot.triple(x) &= j \ dot.quad(x) &= s $

--- math-accent-func paged ---
// Test `accent` function.
$accent(ö, .), accent(v, <-), accent(ZZ, \u{0303})$

--- math-accent-bounds paged ---
// Test accent bounds.
$sqrt(tilde(T)) + hat(f)/hat(g)$

--- math-accent-wide-base paged ---
// Test wide base.
$arrow("ABC" + d), tilde(sum)$

--- math-accent-superscript paged ---
// Test effect of accent on superscript.
$A^x != hat(A)^x != hat(hat(A))^x$

--- math-accent-high-base paged ---
// Test high base.
$ tilde(integral), tilde(integral)_a^b, tilde(integral_a^b) $

--- math-accent-sized paged ---
// Test accent size.
$tilde(sum), tilde(sum, size: #50%), accent(H, hat, size: #200%)$

--- math-accent-sized-script paged ---
// Test accent size in script size.
$tilde(U, size: #1.1em), x^tilde(U, size: #1.1em), sscript(tilde(U, size: #1.1em))$

--- math-accent-dotless paged ---
// Test dotless glyph variants.
#let test(c) = $grave(#c), acute(sans(#c)), hat(frak(#c)), tilde(mono(#c)),
  macron(bb(#c)), dot(cal(#c)), diaer(upright(#c)), breve(bold(#c)),
  circle(bold(upright(#c))), caron(upright(sans(#c))), arrow(bold(frak(#c)))$
$test(i) \ test(j)$

--- math-accent-dotless-disabled paged ---
// Test disabling the dotless glyph variants.
$hat(i), hat(i, dotless: #false), accent(j, tilde), accent(j, tilde, dotless: #false)$

--- math-accent-dotless-set-rule paged ---
#set math.accent(dotless: false)
$ hat(i) $

--- math-accent-dotless-greedy paged ---
// Currently the dotless style propagates to everything in the accent's base,
// even though it shouldn't.
$ arrow(P_(c, i dot j) P_(1, i) j) \
  arrow(P_(c, i dot j) P_(1, i) j, dotless: #false) $

--- math-accent-flattened paged ---
// Test flattened accent glyph variants.
#show math.equation: set text(font: "STIX Two Math")
$hat(a) hat(A)$
$tilde(w) tilde(W)$
$grave(i) grave(j)$
$grave(I) grave(J)$

--- math-accent-bottom paged ---
// Test bottom accents.
$accent(a, \u{20EE}), accent(T, \u{0323}), accent(xi, \u{0332}),
  accent(f, \u{20ED}), accent(F, \u{20E8}), accent(y, \u{032E}),
  accent(!, \u{032F}), accent(J, \u{0333}), accent(p, \u{0331})$

--- math-accent-bottom-wide-base paged ---
// Test wide base with bottom accents.
$accent(x + y, \u{20EF}), accent(sum, \u{032D})$

--- math-accent-bottom-subscript paged ---
// Test effect of bottom accent on subscript.
$q_x != accent(q, \u{032C})_x != accent(accent(q, \u{032C}), \u{032C})_x$

--- math-accent-bottom-high-base paged ---
// Test high base with bottom accents.
$ accent(integral, \u{20EC}), accent(integral, \u{20EC})_a^b, accent(integral_a^b, \u{20EC}) $

--- math-accent-bottom-sized paged ---
// Test bottom accent size.
$accent(sum, \u{0330}), accent(sum, \u{0330}, size: #50%), accent(H, \u{032D}, size: #200%)$

--- math-accent-nested paged ---
// Test nested top and bottom accents.
$hat(accent(L, \u{0330})), accent(circle(p), \u{0323}),
  macron(accent(caron(accent(A, \u{20ED})), \u{0333})) \
  breve(accent(eta, \u{032E})) = accent(breve(eta), \u{032E})$

--- math-accent-string-too-long paged ---
// Error: 17-21 expected exactly one character
$ accent(x + y, "..") $

--- math-accent-content-too-long paged ---
// Error: 17-19 expected a single-codepoint symbol
$ accent(x + y, ..) $

--- issue-7437-math-accent-text-presentation paged ---
// Make sure that the `arrow.l.r` symbol correctly works as an accent even
// though it includes a text presentation variation selector.

// Ensure that symbol style works.
$ accent(x + y, arrow.l.r) $
// Ensure that string style works.
$ accent(x + y, "↔") $
// Ensure that content style works.
$ accent(x + y, ↔) $
// Ensure that shorthand style works.
$ accent(x + y, <->) $
// Ensure that function call works.
$ arrow.l.r(x + y) $

--- issue-7437-math-accent-emoji-presentation paged ---
// Check that we do not normalize an accent character with emoji presentation
// variation selector to an accent.
//
// Since we already support arbitrary characters as accents, we might want to
// support clusters like this one, too, but it should render as an emoji instead
// of normalizing into the same accent as `arrow.l.r`, so it's better to keep
// this an error for now.

// Error: 12-31 expected exactly one character
$accent(A, std.emoji.arrow.l.r)$

--- issue-7437-math-accent-trailing-text paged ---
// Test that we don't allow extra text after the text variation selector.

// Error: 13-47 expected exactly one character
$accent(A, #symbol("\u{2194}\u{fe0e}\u{fe0f}"))$

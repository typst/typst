// Test math accents.

---
// Test function call.
$grave(a), acute(b), hat(f), tilde(§), macron(ä), diaer(a), ä \
 breve(\&), dot(!), circle(a), caron(@), arrow(Z), arrow.l(Z)$

---
$ x &= p \ dot(x) &= v \ dot.double(x) &= a \ dot.triple(x) &= j \ dot.quad(x) &= s $

---
// Test `accent` function.
$accent(ö, .), accent(v, <-), accent(ZZ, \u{0303})$

---
// Test accent bounds.
$sqrt(tilde(T)) + hat(f)/hat(g)$

---
// Test wide base.
$arrow("ABC" + d), tilde(sum)$

---
// Test effect of accent on superscript.
$A^x != hat(A)^x != hat(hat(A))^x$

---
// Test high base.
$ tilde(integral), tilde(integral)_a^b, tilde(integral_a^b) $

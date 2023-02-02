// Test math accents.

---
// Test function call.
$grave(a), acute(b), hat(f), tilde(§), macron(ä), diaer(a), ä, \
 breve(\&), dot(!), circle(a), caron(@), arrow(Z), arrow.l(Z)$

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
// Test high base.
$ tilde(integral), tilde(integral)_a^b, tilde(integral_a^b) $

// test optical sized variants in sub/superscripts

---

// Test transition from script to scriptscript.
#[
#set text(size:20pt)
$  e^(e^(e^(e))) $
]
A large number: $e^(e^(e^(e)))$. 

---
//  Test prime/double prime via scriptsize
#let prime = [ \u{2032} ]
#let dprime = [ \u{2033} ]
#let tprime = [ \u{2034} ]
$ y^dprime-2y^prime + y = 0 $
$y^dprime-2y^prime + y = 0$
$ y^tprime_3 + g^(prime 2) $

---
// Test prime superscript on large symbol 
$ scripts(sum_(k in NN))^prime 1/k^2 $
$sum_(k in NN)^prime 1/k^2$

---
// Test script-script in a fraction.
$ 1/(x^A) $
#[#set text(size:18pt); $1/(x^A)$] vs. #[#set text(size:14pt); $x^A$]

---
// Test dedicated syntax for primes
$a'$, $a'''_b$, $'$, $'''''''$

---
// Test spaces between
$a' ' '$, $' ' '$, $a' '/b$

---
// Test complex prime combilnations
$a'_b^c$, $a_b'^c$, $a_b^c'$, $a_b'^c'^d'$

$(a'_b')^(c'_d')$, $a'/b'$, $a_b'/c_d'$

$∫'$, $∑'$, $ ∑'_S' $

---
// Test attaching primes only
$a' = a^', a_', a_'''^''^'$
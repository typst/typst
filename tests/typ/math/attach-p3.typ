// Test t and b attachments, part 3.

---
// Test limit.
$ lim_(n->oo \ n "grows") sum_(k=0 \ k in NN)^n k $

---
// Test forcing scripts and limits.
$ limits(A)_1^2 != A_1^2 $
$ scripts(sum)_1^2 != sum_1^2 $
$ limits(integral)_a^b != integral_a^b $

---
// Error: 30-34 unknown variable: oops
$ attach(A, t: #locate(it => oops)) $

---
// Show and let rules for limits and scripts
#let eq = $ ∫_a^b iota_a^b $
#eq
#show "∫": math.limits
#show math.iota: math.limits.with(inline: false)
#eq
$iota_a^b$

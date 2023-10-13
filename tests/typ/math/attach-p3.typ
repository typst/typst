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

---
// Test default of limit attachments on relations at all sizes
#set page(width: auto)
$ a =^"def" b quad a lt.eq_"really" b quad  a arrow.r.long.squiggly^"slowly" b $
$a =^"def" b quad a lt.eq_"really" b quad a arrow.r.long.squiggly^"slowly" b$

$a scripts(=)^"def" b quad a scripts(lt.eq)_"really" b quad a scripts(arrow.r.long.squiggly)^"slowly" b$

---
// Test default of scripts attachments on integrals at display size
$ integral.sect_a^b  quad \u{2a1b}_a^b quad limits(\u{2a1b})_a^b $
$integral.sect_a^b quad \u{2a1b}_a^b quad limits(\u{2a1b})_a^b$

---
// Test default of limit attachments on large operators at display size only
$ tack.t.big_0^1 quad \u{02A0A}_0^1 quad join_0^1 $
$tack.t.big_0^1 quad \u{02A0A}_0^1 quad join_0^1$

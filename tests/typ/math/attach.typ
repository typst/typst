// Test top and bottom attachments.

---
// Test basics.
$f_x + t^b + V_1^2
 + attach(A, top: alpha, bottom: beta)$

---
// Test text vs ident parsing.
$pi_1(Y), a_f(x) != a_zeta(x)$

---
// Test associativity and scaling.
$ 1/(V^2^3^4^5) $

---
// Test high subscript and superscript.
$sqrt(a_(1/2)^zeta)$
$sqrt(a_alpha^(1/2))$
$sqrt(a_(1/2)^(3/4))$

---
// Test frame base.
$ (-1)^n + (1/2 + 3)^(-1/2) $

---
// Test limit.
$ lim_(n->infty \ n "grows") sum_(k=0 \ k in NN)^n k $

---
// Test forcing scripts and limits.
$ limits(A)_1^2 != A_1^2 $
$ scripts(sum)_1^2 != sum_1^2 $
$ limits(integral)_a^b != integral_a^b $

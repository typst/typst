// Test top and bottom attachments.

---
// Test basics, postscripts.
$f_x + t^b + V_1^2 + attach(A, top: alpha, bottom: beta)$

---
// Test basics, prescripts.
 $attach(upright(O), prebottom: 8, pretop: 16, bottom: 2, top: 2-),
  attach("Pb", prebottom: 82, pretop: 207) +
  attach(upright(e), prebottom: -1, pretop: 0) + macron(v)_e \
  attach(C, prebottom: n, bottom: r) = mat(n;r),
  attach(a, pretop:k) = a^a^dots.up^a } k$

---
// Test function call after subscript.
$pi_1(Y), a_f(x), a^zeta(x) \
 a^subset.eq(x), a_(zeta(x)), pi_(1(Y))$

---
// Test associativity and scaling.
$ 1/(V^2^3^4^5),
  1/attach(V, pretop: attach(2, pretop: attach(3, pretop: attach(4, pretop: 5)))),
  attach(Omega,
    pretop: attach(2, pretop: attach(3, pretop: attach(4, pretop: 5))),
    top: attach(2, top: attach(3, top: attach(4, top: 5))),
    prebottom: attach(2, prebottom: attach(3, prebottom: attach(4, prebottom: 5))),
    bottom: attach(2, bottom: attach(3, bottom: attach(4, bottom: 5))),
  )
$

---
// Test high subscript and superscript.
$ sqrt(a_(1/2)^zeta), sqrt(a_alpha^(1/2)), sqrt(a_(1/2)^(3/4)) $
$ sqrt(attach(a, pretop: 1/2 alpha, prebottom: 3/4 beta)),
  sqrt(attach(a, pretop: 1/2 alpha, prebottom: 3/4 beta, top: 1/2 alpha, bottom: 3/4 beta)) $

---
// Test frame base.
$ (-1)^n + (1/2 + 3)^(-1/2) $

---
// Test limit.
$ lim_(n->oo \ n "grows") sum_(k=0 \ k in NN)^n k $

---
// Test forcing scripts and limits.
$ limits(A)_1^2 != A_1^2 $
$ scripts(sum)_1^2 != sum_1^2 $
$ limits(integral)_a^b != integral_a^b $

// Test top and bottom attachments.

---
// Test basics, postscripts.
$f_x + t^b + V_1^2 + attach(A, top: alpha, bottom: beta)$

---
// Test basics, prescripts. Notably, the upper and lower prescripts' content need to be
// aligned on the right edge of their bounding boxes, not on the left as in postscripts.
$
attach(upright(O), bottomleft: 8, topleft: 16, bottomright: 2, topright: 2-),
attach("Pb", bottomleft: 82, topleft: 207) +
attach(upright(e), bottomleft: -1, topleft: 0) + macron(v)_e \
attach(C, bottomleft: n, bottomright: r) = mat(n;r),
attach(a, topleft:k) = a^a^dots.up^a } k \
attach(lim, top: n -> 1), attach(lim, bottom: n -> 1),
attach(a, topleft: q, top: "ignored"), attach(a, topright: q, top: "ignored"),
attach(a, bottomleft: q, bottom: "ignored"), attach(a, bottomright: q, bottom: "ignored")
$

---
// Test function call after subscript.
$pi_1(Y), a_f(x), a^zeta(x) \
 a^subset.eq(x), a_(zeta(x)), pi_(1(Y))$

---
// Test associativity and scaling.
$ 1/(V^2^3^4^5),
  1/attach(V, topleft: attach(2, topleft: attach(3, topleft: attach(4, topleft: 5)))),
  attach(Omega,
    topleft: attach(2, topleft: attach(3, topleft: attach(4, topleft: 5))),
    topright: attach(2, topright: attach(3, topright: attach(4, topright: 5))),
    bottomleft: attach(2, bottomleft: attach(3, bottomleft: attach(4, bottomleft: 5))),
    bottomright: attach(2, bottomright: attach(3, bottomright: attach(4, bottomright: 5))),
  )
$

---
// Test high subscript and superscript.
$ sqrt(a_(1/2)^zeta), sqrt(a_alpha^(1/2)), sqrt(a_(1/2)^(3/4)) $
$ sqrt(attach(a, topleft: 1/2 alpha, bottomleft: 3/4 beta)),
  sqrt(attach(a, topleft: 1/2 alpha, bottomleft: 3/4 beta, topright: 1/2 alpha, bottomright: 3/4 beta)) $

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

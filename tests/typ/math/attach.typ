// Test t and b attachments.

---
// Test basics, postscripts.
$f_x + t^b + V_1^2 + attach(A, t: alpha, b: beta)$

---
// Test basics, prescripts. Notably, the upper and lower prescripts' content need to be
// aligned on the right edge of their bounding boxes, not on the left as in postscripts.
$
attach(upright(O), bl: 8, tl: 16, br: 2, tr: 2-),
attach("Pb", bl: 82, tl: 207) +
attach(upright(e), bl: -1, tl: 0) + macron(v)_e \
attach(C, bl: n, br: r) = mat(n;r),
attach(a, tl:k) = a^a^dots.up^a } k \
attach(lim, t: n -> 1), attach(lim, b: n -> 1),
attach(a, tl: q, t: "ignored"), attach(a, tr: q, t: "ignored"),
attach(a, bl: q, b: "ignored"), attach(a, br: q, b: "ignored")
$

---
// Test function call after subscript.
$pi_1(Y), a_f(x), a^zeta(x) \
 a^subset.eq(x), a_(zeta(x)), pi_(1(Y))$

---
// Test associativity and scaling.
$ 1/(V^2^3^4^5),
  1/attach(V, tl: attach(2, tl: attach(3, tl: attach(4, tl: 5)))),
  attach(Omega,
    tl: attach(2, tl: attach(3, tl: attach(4, tl: 5))),
    tr: attach(2, tr: attach(3, tr: attach(4, tr: 5))),
    bl: attach(2, bl: attach(3, bl: attach(4, bl: 5))),
    br: attach(2, br: attach(3, br: attach(4, br: 5))),
  )
$

---
// Test high subscript and superscript.
$ sqrt(a_(1/2)^zeta), sqrt(a_alpha^(1/2)), sqrt(a_(1/2)^(3/4)) $
$ sqrt(attach(a, tl: 1/2 alpha, bl: 3/4 beta)),
  sqrt(attach(a, tl: 1/2 alpha, bl: 3/4 beta, tr: 1/2 alpha, br: 3/4 beta)) $

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

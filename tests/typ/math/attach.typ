// Test t and b attachments.

---
// Test basics, postscripts.
$f_x + t^b + V_1^2 + attach(A, t: alpha, b: beta)$

---
// Test basics, prescripts. Notably, the upper and lower prescripts' content need to be
// aligned on the right edge of their bounding boxes, not on the left as in postscripts.
$
attach(upright(O), bl: 8, tl: 16, br: 2, tr: 2-),
attach("Pb", bl: 82, tl: 207) + attach(upright(e), bl: -1, tl: 0) + macron(v)_e \
$

---
// A mixture of attachment positioning schemes.
$
attach(a, tl: u),   attach(a, tr: v),   attach(a, bl: x),
attach(a, br: y),   limits(a)^t,        limits(a)_b \

attach(a, tr: v, t: t),
attach(a, tr: v, br: y),
attach(a, br: y, b: b),
attach(limits(a), b: b, bl: x),
attach(a, tl: u, bl: x),
attach(limits(a), t: t, tl: u) \

attach(a, tl: u, tr: v),
attach(limits(a), t: t, br: y),
attach(limits(a), b: b, tr: v),
attach(a, bl: x, br: y),
attach(limits(a), b: b, tl: u),
attach(limits(a), t: t, bl: u),
limits(a)^t_b \

attach(a, tl: u, tr: v, bl: x, br: y),
attach(limits(a), t: t, bl: x, br: y, b: b),
attach(limits(a), t: t, tl: u, tr: v, b: b),
attach(limits(a), tl: u, bl: x, t: t, b: b),
attach(limits(a), t: t, b: b, tr: v, br: y),
attach(a, tl: u, t: t, tr: v, bl: x, b: b, br: y)
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
$ sqrt(a_(1/2)^zeta), sqrt(a_alpha^(1/2)), sqrt(a_(1/2)^(3/4)) \
  sqrt(attach(a, tl: 1/2, bl: 3/4)),
  sqrt(attach(a, tl: 1/2, bl: 3/4, tr: 1/2, br: 3/4)) $

---
// Test frame base.
$ (-1)^n + (1/2 + 3)^(-1/2) $

---
#set text(size: 8pt)

// Test that the attachments are aligned horizontally.
$ x_1 p_1 frak(p)_1 2_1 dot_1 lg_1 !_1 \\_1 ]_1 "ip"_1 op("iq")_1 \
  x^1 b^1 frak(b)^1 2^1 dot^1 lg^1 !^1 \\^1 ]^1 "ib"^1 op("id")^1 \
  x_1 y_1 "_"_1 x^1 l^1 "`"^1 attach(I,tl:1,bl:1,tr:1,br:1)
  scripts(sum)_1^1 integral_1^1 |1/2|_1^1 \
  x^1_1, "("b y")"^1_1 != (b y)^1_1, "[∫]"_1 [integral]_1 $

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

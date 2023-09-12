// Test t and b attachments, part 1.

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
$pi_1(Y), a_f(x), a^zeta (x), a^abs(b)_sqrt(c) \
 a^subset.eq (x), a_(zeta(x)), pi_(1(Y)), a^(abs(b))_(sqrt(c))$

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

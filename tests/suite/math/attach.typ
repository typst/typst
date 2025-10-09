// Test t and b attachments, part 1.

--- math-attach-postscripts ---
// Test basics, postscripts.
$f_x + t^b + V_1^2 + attach(A, t: alpha, b: beta)$

--- math-attach-prescripts ---
// Test basics, prescripts. Notably, the upper and lower prescripts' content need to be
// aligned on the right edge of their bounding boxes, not on the left as in postscripts.
$
attach(upright(O), bl: 8, tl: 16, br: 2, tr: 2-),
attach("Pb", bl: 82, tl: 207) + attach(upright(e), bl: -1, tl: 0) + macron(v)_e \
$

--- math-attach-mixed ---
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

--- math-attach-followed-by-func-call ---
// Test function call after subscript.
$pi_1(Y), a_f(x), a^zeta (x), a^abs(b)_sqrt(c) \
 a^subset.eq (x), a_(zeta(x)), pi_(1(Y)), a^(abs(b))_(sqrt(c))$

--- math-attach-nested ---
// Test associativity and scaling.
$ 1/(V^2^3^4^5),
  frac(
    attach(
      limits(V), br: attach(2, br: 3), b: attach(limits(2), b: 3)),
    attach(
      limits(V), tl: attach(2, tl: 3), t: attach(limits(2), t: 3))),
  attach(Omega,
    tl: attach(2, tl: attach(3, tl: attach(4, tl: 5))),
    tr: attach(2, tr: attach(3, tr: attach(4, tr: 5))),
    bl: attach(2, bl: attach(3, bl: attach(4, bl: 5))),
    br: attach(2, br: attach(3, br: attach(4, br: 5))),
  )
$

--- math-attach-high ---
// Test high subscript and superscript.
$ sqrt(a_(1/2)^zeta), sqrt(a_alpha^(1/2)), sqrt(a_(1/2)^(3/4)) \
  sqrt(attach(a, tl: 1/2, bl: 3/4)),
  sqrt(attach(a, tl: 1/2, bl: 3/4, tr: 1/2, br: 3/4)) $

--- math-attach-descender-collision ---
// Test for no collisions between descenders/ascenders and attachments.

$ sup_(x in P_i) quad inf_(x in P_i) $
$ op("fff",limits: #true)^(y) quad op("yyy", limits:#true)_(f) $

--- math-attach-to-group ---
// Test frame base.
$ (-1)^n + (1/2 + 3)^(-1/2) $

--- math-attach-horizontal-align ---
#set text(size: 8pt)

// Test that the attachments are aligned horizontally.
$ x_1 p_1 frak(p)_1 2_1 dot_1 lg_1 !_1 \\_1 ]_1 "ip"_1 op("iq")_1 \
  x^1 b^1 frak(b)^1 2^1 dot^1 lg^1 !^1 \\^1 ]^1 "ib"^1 op("id")^1 \
  "_"_1 "`"^1 x_1 y_1 x^1 l^1 attach(I,tl:1,bl:1,tr:1,br:1)
  scripts(sum)_1^1 integral_1^1 abs(1/2)_1^1 \
  x^1_1, ")"^1_1 (b y)^1_1, "[∫]"_1 [integral]_1 $

--- math-attach-limit ---
// Test limit.
$ lim_(n->oo \ n "grows") sum_(k=0 \ k in NN)^n k $

--- math-attach-force-scripts-and-limits ---
// Test forcing scripts and limits.
$ limits(A)_1^2 != A_1^2 $
$ scripts(sum)_1^2 != sum_1^2 $
$ limits(integral)_a^b != integral_a^b $

--- issue-math-attach-realize-panic ---
// Error: 25-29 unknown variable: oops
$ attach(A, t: #context oops) $

--- math-attach-show-limit ---
// Show and let rules for limits and scripts
#let eq = $ ∫_a^b iota_a^b $
#eq
#show "∫": math.limits
#show math.iota: math.limits.with(inline: false)
#eq
$iota_a^b$

--- math-attach-default-placement ---
// Test default of limit attachments on relations at all sizes.
#set page(width: auto)
$ a =^"def" b quad a lt.eq_"really" b quad  a arrow.r.long.squiggly^"slowly" b $
$a =^"def" b quad a lt.eq_"really" b quad a arrow.r.long.squiggly^"slowly" b$

$a scripts(=)^"def" b quad a scripts(lt.eq)_"really" b quad a scripts(arrow.r.long.squiggly)^"slowly" b$

--- math-attach-integral ---
// Test default of scripts attachments on integrals at display size.
$ integral.inter_a^b  quad \u{2a1b}_a^b quad limits(\u{2a1b})_a^b $
$integral.inter_a^b quad \u{2a1b}_a^b quad limits(\u{2a1b})_a^b$

--- math-attach-large-operator ---
// Test default of limit attachments on large operators at display size only.
$ tack.t.big_0^1 quad \u{02A0A}_0^1 quad join_0^1 $
$tack.t.big_0^1 quad \u{02A0A}_0^1 quad join_0^1$

--- math-attach-limit-long ---
// Test long limit attachments.
$ attach(product, t: 123456789) attach(product, t: 123456789, bl: x) \
  attach(product, b: 123456789) attach(product, b: 123456789, tr: x) $
$attach(limits(product), t: 123456789) attach(limits(product), t: 123456789, bl: x)$

$attach(limits(product), b: 123456789) attach(limits(product), b: 123456789, tr: x)$

--- math-attach-kerning ---
// Test math kerning.
#show math.equation: set text(font: "STIX Two Math")

$ L^A Y^c R^2 delta^y omega^f a^2 t^w gamma^V p^+ \
  b_lambda f_k p_i x_1 x_j x_A y_l y_y beta_s theta_k \
  J_0 Y_0 T_1 T_f V_a V_A F_j cal(F)_j lambda_y \
  attach(W, tl: l) attach(A, tl: 2) attach(cal(V), tl: beta)
  attach(cal(P), tl: iota) attach(f, bl: i) attach(A, bl: x)
  attach(cal(J), bl: xi) attach(cal(A), bl: m) $

--- math-attach-kerning-mixed ---
// Test mixtures of math kerning.
#show math.equation: set text(font: "STIX Two Math")

$ x_1^i x_2^lambda x_2^(2alpha) x_2^(k+1) x_2^(-p_(-1)) x_j^gamma \
  f_2^2 v_0^2  z_0^2 beta_s^2 xi_i^k J_1^2 N_(k y)^(-1) V_pi^x \
  attach(J, tl: 1, br: i) attach(P, tl: i, br: 2) B_i_0 phi.alt_i_(n-1)
  attach(A, tr: x, bl: x, br: x, tl: x) attach(F, tl: i, tr: f) \
  attach(cal(A), tl: 2, bl: o) attach(cal(J), bl: l, br: A)
  attach(cal(y), tr: p, bl: n t) attach(cal(O), tl: 16, tr: +, br: sigma)
  attach(italic(Upsilon), tr: s, br: Psi, bl: d) $

--- math-attach-nested-base ---
// Test attachments when the base has attachments.
$ attach(a^b, b: c) quad
  attach(attach(attach(attach(attach(attach(sum, tl: 1), t: 2), tr: 3), br: 4), b: 5), bl: 6) $

#let a0 = math.attach(math.alpha, b: [0])
#let a1 = $alpha^1$
#let a2 = $attach(a1, bl: 3)$

$ a0 + a1 + a0_2 \
  a1_2 + a0^2 + a1^2 \
  a2 + a2_2 + a2^2 $

--- math-attach-nested-deep-base ---
// Test attachments when the base has attachments and is nested arbitrarily
// deep.
#{
  let var = $x^1$
  for i in range(24) {
    var = $var$
  }
  $var_2$
}

--- math-attach-scripts-extended-shapes ---
// Test script attachments positioning if the base is an extended shape (or a
// sequence of extended shapes).
$lr(size: #130%, [x])_0^1, [x]_0^1, \]_0^1, x_0^1, A_0^1$ \
$n^2, (n + 1)^2, sum_0^1, integral_0^1$

--- math-attach-missing-sides ---
// Test attachments that are missing a side.
// Error: 23-24 unexpected underscore
$ a _ b (d _) (d'_ ) (_ c) $


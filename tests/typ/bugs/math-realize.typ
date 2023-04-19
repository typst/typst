// Test that content in math can be realized without breaking
// nested equations.

---
#let my = $pi$
#let f1 = box(baseline: 10pt, [f])
#let f2 = style(sty => f1)
#show math.vec: [nope]

$ pi a $
$ my a $
$ 1 + sqrt(x/2) + sqrt(#hide($x/2$)) $
$ a x #link("url", $+ b$) $
$ f f1 f2 $
$ vec(1,2) * 2 $

---
$ x^2 #hide[$(>= phi.alt) union y^2 0$] z^2 $
Hello #hide[there $x$]
and #hide[$ f(x) := x^2 $]

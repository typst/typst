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

---
// Test equations can embed equation pieces built by functions
#let foo(upper, lower) = {
  // Return an equation piece that would've been rendered in
  // inline style if the piece is not embedded
  $(upper/lower)^2$
}
#let bar(upper, lower) = {
  // Return an equation piece that would've been rendered in
  // block style if the piece is not embedded
  $ (upper/lower)^2 $
}

Inline $foo(alpha, M+1)+sigma^2$.

Inline $bar(alpha, M+1)+sigma^2$.

Block:
$ foo(alpha, M+1)+sigma^2 $
Block:
$ bar(alpha, M+1)+sigma^2 $

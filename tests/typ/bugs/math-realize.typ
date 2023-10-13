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
#let foo(v1, v2) = {
  // Return an equation piece that would've been rendered in
  // inline style if the piece is not embedded
  $v1 v2^2$
}
#let bar(v1, v2) = {
  // Return an equation piece that would've been rendered in
  // block style if the piece is not embedded
  $ v1 v2^2 $
}
#let baz(..sink) = {
  // Return an equation piece built by joining arrays
  sink.pos().map(x => $hat(#x)$).join(sym.and)
}

Inline $2 foo(alpha, (M+foo(a, b)))$.

Inline $2 bar(alpha, (M+foo(a, b)))$.

Inline $2 baz(x,y,baz(u, v))$.

$ 2 foo(alpha, (M+foo(a, b))) $
$ 2 bar(alpha, (M+foo(a, b))) $
$ 2 baz(x,y,baz(u, v)) $

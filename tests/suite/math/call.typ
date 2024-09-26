// Test math function call edge cases.

// Note: 2d argument calls are tested for matrices in `mat.typ`

--- math-call-non-func ---
$ pi(a) $
$ pi(a,) $
$ pi(a,b) $
$ pi(a,b,) $

--- math-call-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a)$, "($var(\"a\")$)")
#check($args(a,)$, "($var(\"a\")$)")
#check($args(a,b)$, "($var(\"a\")$, $var(\"b\")$)")
#check($args(a,b,)$, "($var(\"a\")$, $var(\"b\")$)")
#check($args(,a,b,,,)$, "([], $var(\"a\")$, $var(\"b\")$, [], [])")

--- math-call-2d-non-func ---
// Error: 6-7 expected content, found array
// Error: 8-9 expected content, found array
$ pi(a;b) $

--- math-call-2d-semicolon-priority ---
// If the semicolon directly follows a hash expression, it terminates that
// instead of indicating 2d arguments.
$ mat(#"math" ; "wins") $
$ mat(#"code"; "wins") $

--- math-call-2d-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a;b)$, "(($var(\"a\")$,), ($var(\"b\")$,))")
#check($args(a,b;c)$, "(($var(\"a\")$, $var(\"b\")$), ($var(\"c\")$,))")
#check($args(a,b;c,d;e,f)$, "(\n  ($var(\"a\")$, $var(\"b\")$),\n  ($var(\"c\")$, $var(\"d\")$),\n  ($var(\"e\")$, $var(\"f\")$),\n)")

--- math-call-2d-repr-structure ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args( a; b; )$, "(($var(\"a\")$,), ($var(\"b\")$,))")
#check($args(a;  ; c)$, "(($var(\"a\")$,), ([],), ($var(\"c\")$,))")
#check($args(a b,/**/; b)$, "(\n  (sequence($var(\"a\")$, [ ], $var(\"b\")$), []),\n  ($var(\"b\")$,),\n)")
#check($args(a/**/b, ; b)$, "(\n  (sequence($var(\"a\")$, $var(\"b\")$), []),\n  ($var(\"b\")$,),\n)")
#check($args( ;/**/a/**/b/**/; )$, "(([],), (sequence($var(\"a\")$, $var(\"b\")$),))")
#check($args( ; , ; )$, "(([],), ([], []))")
#check($args(/**/; // funky whitespace/trivia
    ,   /**/  ;/**/)$, "(([],), ([], []))")

--- math-call-empty-args-non-func ---
// Trailing commas and empty args introduce blank content in math.
$ sin(,x,y,,,) $
// with whitespace/trivia:
$ sin( ,/**/x/**/, , /**/y, ,/**/, ) $

--- math-call-empty-args-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(,x,,y,,)$, "([], $var(\"x\")$, [], $var(\"y\")$, [])")
// with whitespace/trivia:
#check($args( ,/**/x/**/, , /**/y, ,/**/, )$, "([], $var(\"x\")$, [], $var(\"y\")$, [], [])")

--- math-call-value-non-func ---
$ sin(1) $
// Error: 8-9 expected content, found integer
$ sin(#1) $

--- math-call-pass-to-box ---
// This test was fixed by PR 4638 adding math.var! See there for details.

// When passing to a function, we used to lose the italic styling if the content
// was wrapped in a non-math function (i.e. box) unless it was already nested in
// some math element (lr, attach, etc.)
#let id(body) = body
#let eq(body) = math.equation(body)
#let bx(body) = box(body, stroke: blue+0.5pt, inset: (x:2pt, y:3pt))
$
     x y   &&quad     x (y z)   &quad     x y^z  \
  id(x y)  &&quad  id(x (y z))  &quad  id(x y^z) \
  eq(x y)  &&quad  eq(x (y z))  &quad  eq(x y^z) \
  bx(x y)  &&quad  bx(x (y z))  &quad  bx(x y^z) \
$

--- math-call-unknown-var-hint ---
// Error: 4-6 unknown variable: ab
// Hint: 4-6 if you meant to display multiple letters as is, try adding spaces between each letter: `a b`
// Hint: 4-6 or if you meant to display this as text, try placing it in quotes: `"ab"`
$ 5ab $

--- issue-3774-math-call-empty-2d-args ---
$ mat(;,) $
// Add some whitespace/trivia:
$ mat(; ,) $
$ mat(;/**/,) $
$ mat(;
,) $
$ mat(;// line comment
,) $
$ mat(
  1, , ;
   ,1, ;
   , ,1;
) $

--- issue-2885-math-var-only-in-global ---
// Error: 7-10 unknown variable: rgb
// Hint: 7-10 `rgb` is not available directly in math, try adding a hash before it: `#rgb`
$text(rgb(0, 0, 0), "foo")$

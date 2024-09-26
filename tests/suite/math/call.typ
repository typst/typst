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
#check($args(a)$, "([a])")
#check($args(a,)$, "([a])")
#check($args(a,b)$, "([a], [b])")
#check($args(a,b,)$, "([a], [b])")
#check($args(,a,b,,,)$, "([], [a], [b], [], [])")

--- math-call-2d-non-func ---
// Error: 6-7 expected content, found array
// Error: 8-9 expected content, found array
$ pi(a;b) $

--- math-call-2d-semicolon-priority ---
// If the semicolon directlry follows a hash expression, it terminates that
// instead of indicating 2d arguments.
$ mat(#"math" ; "wins") $
$ mat(#"code"; "wins") $

--- math-call-2d-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a;b)$, "(([a],), ([b],))")
#check($args(a,b;c)$, "(([a], [b]), ([c],))")
#check($args(a,b;c,d;e,f)$, "(([a], [b]), ([c], [d]), ([e], [f]))")

--- math-call-2d-repr-structure ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args( a; b; )$, "(([a],), ([b],))")
#check($args(a;  ; c)$, "(([a],), ([],), ([c],))")
#check($args(a b,/**/; b)$, "((sequence([a], [ ], [b]), []), ([b],))")
#check($args(a/**/b, ; b)$, "((sequence([a], [b]), []), ([b],))")
#check($args( ;/**/a/**/b/**/; )$, "(([],), (sequence([a], [b]),))")
#check($args( ; , ; )$, "(([],), ([], []))")
#check($args(/**/; // funky whitespace/trivia
    ,   /**/  ;/**/)$, "(([],), ([], []))")

--- math-call-empty-args-non-func ---
// Trailing commas and empty args introduce blank content in math
$ sin(,x,y,,,) $
// with whitespace/trivia:
$ sin( ,/**/x/**/, , /**/y, ,/**/, ) $

--- math-call-empty-args-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(,x,,y,,)$, "([], [x], [], [y], [])")
// with whitespace/trivia:
#check($args( ,/**/x/**/, , /**/y, ,/**/, )$, "([], [x], [], [y], [], [])")

--- math-call-value-non-func ---
$ sin(1) $
// Error: 8-9 expected content, found integer
$ sin(#1) $

--- math-call-pass-to-box ---
// When passing to a function, we lose the italic styling if we wrap the content
// in a non-math function unless it's already nested in some math element (lr,
// attach, etc.)
//
// This is not good, so this test should fail and be updated once it is fixed.
#let id(body) = body
#let bx(body) = box(body, stroke: blue+0.5pt, inset: (x:2pt, y:3pt))
#let eq(body) = math.equation(body)
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

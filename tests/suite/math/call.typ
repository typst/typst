// Test math function call edge cases.

// Note: 2d argument calls are tested for matrices in `mat.typ`

--- math-call-non-func ---
$ pi(a) $
$ pi(a,) $
$ pi(a,b) $
$ pi(a,b,) $

--- math-call-repr ---
#let viewRepr(..body) = body
$ viewRepr(a) $
$ viewRepr(a,) $
$ viewRepr(a,b) $
$ viewRepr(a,b,) $
$ viewRepr(,a,b,,,) $

--- math-call-2d-non-func ---
// Error: 6-7 expected content, found array
// Error: 8-9 expected content, found array
$ pi(a;b) $

--- math-call-2d-semicolon-priority ---
$ mat("math" ; "wins") $
$ mat(#"code"; "wins") $

--- math-call-2d-repr ---
#let viewRepr(..body) = body
$ viewRepr(a;b) $
$ viewRepr(a,b;c) $
$ viewRepr(a,b;c,d;e,f) $

--- math-call-empty-args-non-func ---
// Trailing commas and empty args introduce blank content in math
$ sin(,x,y,,,) $
// with whitespace/trivia:
$ sin( ,/**/x/**/, , /**/y, ,/**/, ) $

--- math-call-empty-args-repr ---
#let viewRepr(..body) = body
$ viewRepr(,x,,y,,) $
// with whitespace/trivia:
$ viewRepr( ,/**/x/**/, , /**/y, ,/**/, ) $

--- math-call-empty-2d-args-issue-3774 ---
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

--- math-call-2d-repr-structure ---
#set page(width: auto)
#let viewRepr(..body) = body;
$ viewRepr( a; b; ) $
$ viewRepr(a;  ; c) $
$ viewRepr(a b,/**/; b) $
$ viewRepr(a/**/b, ; b) $
$ viewRepr( ;/**/a/**/b/**/; ) $
$ viewRepr( ; , ; ) $
$ viewRepr(/**/; // funky whitespace/trivia
    ,   /**/  ;/**/) $

--- math-call-value-non-func ---
$ sin(1) $
// Error: 8-9 expected content, found integer
$ sin(#1) $

--- math-call-pass-to-box ---
// When passing to a function, we lose the italic styling if we wrap
// the content in a non-math function unless it's already nested in
// some math element (lr, attach, etc.)
#let id(body) = body
#let bx(body) = box(body, stroke: blue+0.5pt, inset: (x:2pt, y:3pt))
#let eq(body) = math.equation(body)
$
     x y   &&quad     x (y z)   &quad     x y^z   \
  id(x y)  &&quad  id(x (y z))  &quad  id(x y^z)  \
  bx(x y)  &&quad  bx(x (y z))  &quad  bx(x y^z)  \
  eq(x y)  &&quad  eq(x (y z))  &quad  eq(x y^z)  \
$

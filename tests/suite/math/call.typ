// Test math function call edge cases.

// Note: 2d argument calls are tested for matrices in `mat.typ`

--- math-call-non-func ---
$ pi(a) $
$ pi(a,) $
$ pi(a,b) $
$ pi(a,b,) $

--- math-call-unclosed-func ---
#let func(x) = x
// Error: 6-7 unclosed delimiter
$func(a$

--- math-call-unclosed-non-func ---
// Error: 5-6 unclosed delimiter
$sin(x$

--- math-call-named-args ---
#let func1(my: none) = my
#let func2(_my: none) = _my
#let func3(my-body: none) = my-body
#let func4(_my-body: none) = _my-body
#let func5(m: none) = m
$ func1(my: a) $
$ func2(_my: a) $
$ func3(my-body: a) $
$ func4(_my-body: a) $
$ func5(m: a) $
$ func5(m: sigma : f) $
$ func5(m: sigma:pi) $

--- math-call-named-args-no-expr ---
#let func(m: none) = m
// Error: 10 expected expression
$ func(m: ) $

--- math-call-named-args-duplicate ---
#let func(my: none) = my
// Error: 15-17 duplicate argument: my
$ func(my: a, my: b) $

--- math-call-named-args-shorthand-clash-1 ---
#let func(m: none) = m
// Error: 18-21 unexpected argument
$func(m: =) func(m:=)$

--- math-call-named-args-shorthand-clash-2 ---
#let func(m: none) = m
// Error: 41-45 unexpected argument
$func(m::) func(m: :=) func(m:: =) func(m::=)$

--- math-call-named-single-underscore ---
#let func(x) = x
// Error: 8-9 expected identifier, found underscore
$ func(_: a) $

--- math-call-named-single-char-error ---
#let func(m: none) = m
// Error: 8-13 unexpected argument
$ func(m : a) $

--- math-call-named-args-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(_a: a)$, "arguments(_a: [a])")
#check($args(_a-b: a)$, "arguments(_a-b: [a])")
#check($args(a-b: a)$, "arguments(a-b: [a])")
#check($args(a-b-c: a)$, "arguments(a-b-c: [a])")
#check($args(a--c: a)$, "arguments(a--c: [a])")
#check($args(a: a-b)$, "arguments(a: sequence([a], [−], [b]))")
#check($args(a-b: a-b)$, "arguments(a-b: sequence([a], [−], [b]))")
#check($args(a-b)$, "arguments(sequence([a], [−], [b]))")

--- math-call-spread-content-error ---
#let args(..body) = body
// Error: 7-16 cannot spread content
$args(..(a + b))$

--- math-call-spread-multiple-exprs ---
#let args(..body) = body
// Error: 10 expected comma or semicolon
$args(..a + b)$

--- math-call-spread-unexpected-dots ---
#let args(..body) = body
// Error: 8-10 unexpected dots
$args(#..range(1, 5).chunks(2))$

--- math-call-spread-shorthand-clash ---
#let func(body) = body
$func(...)$

--- math-call-spread-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(..#range(0, 4).chunks(2))$, "arguments((0, 1), (2, 3))")
#check($#args(range(1, 5).chunks(2))$, "arguments(((1, 2), (3, 4)))")
#check($#args(..range(1, 5).chunks(2))$, "arguments((1, 2), (3, 4))")
#check($args(#(..range(2, 6).chunks(2)))$, "arguments(((2, 3), (4, 5)))")
#let nums = range(0, 4).chunks(2)
#check($args(..nums)$, "arguments((0, 1), (2, 3))")
#check($args(..nums;)$, "arguments(((0, 1), (2, 3)))")
#check($args(..nums, ..nums)$, "arguments((0, 1), (2, 3), (0, 1), (2, 3))")
#check($args(..nums, 4, 5)$, "arguments((0, 1), (2, 3), [4], [5])")
#check($args(..nums, ..#range(4, 6))$, "arguments((0, 1), (2, 3), 4, 5)")
#check($args(..nums, #range(4, 6))$, "arguments((0, 1), (2, 3), (4, 5))")
#check($args(..nums, 1, 2; 3, 4)$, "arguments(((0, 1), (2, 3), [1], [2]), ([3], [4]))")
#check($args(1, 2; ..nums)$, "arguments(([1], [2]), ((0, 1), (2, 3)))")
#check($args(1, 2; 3, 4)$, "arguments(([1], [2]), ([3], [4]))")
#check($args(1, 2; 3, 4; ..#range(5, 7))$, "arguments(([1], [2]), ([3], [4]), (5, 6))")
#check($args(1, 2; 3, 4, ..#range(5, 7))$, "arguments(([1], [2]), ([3], [4], 5, 6))")
#check($args(1, 2; 3, 4, ..#range(5, 7);)$, "arguments(([1], [2]), ([3], [4], 5, 6))")
#check($args(1, 2; 3, 4, ..#range(5, 7),)$, "arguments(([1], [2]), ([3], [4], 5, 6))")

--- math-call-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a)$, "arguments([a])")
#check($args(a,)$, "arguments([a])")
#check($args(a,b)$, "arguments([a], [b])")
#check($args(a,b,)$, "arguments([a], [b])")
#check($args(,a,b,,,)$, "arguments([], [a], [b], [], [])")

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
#check($args(a;b)$, "arguments(([a],), ([b],))")
#check($args(a,b;c)$, "arguments(([a], [b]), ([c],))")
#check($args(a,b;c,d;e,f)$, "arguments(([a], [b]), ([c], [d]), ([e], [f]))")

--- math-call-2d-named-repr ---
#let args(..body) = (body.pos(), body.named())
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a: b)$, "((), (a: [b]))")
#check($args(1, 2; 3, 4)$, "((([1], [2]), ([3], [4])), (:))")
#check($args(a: b, 1, 2; 3, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, a: b, 2; 3, 4)$, "(([1], ([2],), ([3], [4])), (a: [b]))")
#check($args(1, 2, a: b; 3, 4)$, "(([1], [2], (), ([3], [4])), (a: [b]))")
#check($args(1, 2; a: b, 3, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, 2; 3, a: b, 4)$, "((([1], [2]), [3], ([4],)), (a: [b]))")
#check($args(1, 2; 3, 4, a: b)$, "((([1], [2]), [3], [4]), (a: [b]))")
#check($args(a: b, 1, 2, 3, c: d)$, "(([1], [2], [3]), (a: [b], c: [d]))")
#check($args(1, 2, 3; a: b)$, "((([1], [2], [3]),), (a: [b]))")
#check($args(a-b: a,, e:f;; d)$, "(([], (), ([],), ([d],)), (a-b: [a], e: [f]))")
#check($args(a: b, ..#range(0, 4))$, "((0, 1, 2, 3), (a: [b]))")

--- math-call-2d-escape-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a\;b)$, "arguments(sequence([a], [;], [b]))")
#check($args(a\,b;c)$, "arguments((sequence([a], [,], [b]),), ([c],))")
#check($args(b\;c\,d;e)$, "arguments((sequence([b], [;], [c], [,], [d]),), ([e],))")
#check($args(a\: b)$, "arguments(sequence([a], [:], [ ], [b]))")
#check($args(a : b)$, "arguments(sequence([a], [ ], [:], [ ], [b]))")
#check($args(\..a)$, "arguments(sequence([.], [.], [a]))")
#check($args(.. a)$, "arguments(sequence([.], [.], [ ], [a]))")
#check($args(a..b)$, "arguments(sequence([a], [.], [.], [b]))")

--- math-call-2d-repr-structure ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args( a; b; )$, "arguments(([a],), ([b],))")
#check($args(a;  ; c)$, "arguments(([a],), ([],), ([c],))")
#check($args(a b,/**/; b)$, "arguments((sequence([a], [ ], [b]), []), ([b],))")
#check($args(a/**/b, ; b)$, "arguments((sequence([a], [b]), []), ([b],))")
#check($args( ;/**/a/**/b/**/; )$, "arguments(([],), (sequence([a], [b]),))")
#check($args( ; , ; )$, "arguments(([],), ([], []))")
#check($args(/**/; // funky whitespace/trivia
    ,   /**/  ;/**/)$, "arguments(([],), ([], []))")

--- math-call-empty-args-non-func ---
// Trailing commas and empty args introduce blank content in math.
$ sin(,x,y,,,) $
// with whitespace/trivia:
$ sin( ,/**/x/**/, , /**/y, ,/**/, ) $

--- math-call-empty-args-repr ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(,x,,y,,)$, "arguments([], [x], [], [y], [])")
// with whitespace/trivia:
#check($args( ,/**/x/**/, , /**/y, ,/**/, )$, "arguments([], [x], [], [y], [], [])")

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

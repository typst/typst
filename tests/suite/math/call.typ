// Test function calls in math.

// Tests for two-dimensional arguments in matrices are in `mat.typ`.

// Tests for method calls in math are in `../scripting/methods.typ` as
// `math-field-call-*`.

--- math-call-non-func paged ---
// Using call syntax with a non-function in math renders the callee next to
// parens by "unparsing" the arguments into content.
$ phi(x) $
$ phi(x, y, 1/2) $

--- math-call-non-func-empty-args paged ---
// Trailing commas and empty args introduce blank content in math
$ sin(,x,y,,,) $
// with whitespace/trivia:
$ sin( ,/**/x/**/, , /**/y, ,/**/, ) $

--- math-call-non-func-named eval ---
// Hint: 3-6 `phi` is not a function
// Error: 10-18 named-argument syntax can only be used with functions
// Hint: 10-18 to render the colon as text, escape it: `alpha\: y`
$ phi(x, alpha: y) $

--- math-call-non-func-spread eval ---
// Hint: 3-6 `phi` is not a function
// Error: 10-17 spread-argument syntax can only be used with functions
// Hint: 10-17 to render the dots as text, add a space: `.. alpha`
$ phi(x, ..alpha) $

--- math-call-non-func-2d paged ---
$ pi(a;b) quad gamma(;) quad eta(#"a";;, ; upright(b),) $

--- math-call-non-func-spacing paged ---
// Test that we keep the same spacing when unparsing.
#show regex("[,;]"): math.class.with("fence")
$ phi(| , | ; |) \
  phi/**/(| , | ; |) $
#test($     phi(| , | ; |) $,
      $ phi/**/(| , | ; |) $)

--- math-call-unclosed-func eval ---
#let func(x) = x
// Error: 6-7 unclosed delimiter
$func(a$

--- math-call-unclosed-non-func eval ---
// Error: 5-6 unclosed delimiter
$sin(x$

--- math-call-missing-operator-sides eval ---
// Error: 6-7 unclosed delimiter
$func(1^2_, 1/; , , √ )$

--- math-call-basic eval ---
// Function arguments in math.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a)$, "arguments([a])")
#check($args(a,)$, "arguments([a])")
#check($args(a,b)$, "arguments([a], [b])")
#check($args(a,b,)$, "arguments([a], [b])")

--- math-call-empty-args eval ---
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(,x,,y,,)$, "arguments([], [x], [], [y], [])")
// with whitespace/trivia:
#check($args( ,/**/x/**/, , /**/y, ,/**/, )$, "arguments([], [x], [], [y], [], [])")

--- math-call-escape eval ---
// Escaped characters in calls.
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

--- math-call-pass-to-box paged ---
// When passing to a function, we lose the italic styling if we wrap the content
// in a non-math function, unless it's already nested in some math element (lr,
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

--- math-call-named-basic eval ---
// Named arguments in math.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(_a: a)$, "arguments(_a: [a])")
#check($args(_a-b: a)$, "arguments(_a-b: [a])")
#check($args(a-b: a)$, "arguments(a-b: [a])")
#check($args(a-b-c: a)$, "arguments(a-b-c: [a])")
#check($args(a--c: a)$, "arguments(a--c: [a])")
#check($args(a: a-b)$, "arguments(a: sequence([a], [−], [b]))")
#check($args(a: a:b)$, "arguments(a: sequence([a], [:], [b]))")
#check($args(a: a : b)$, "arguments(a: sequence([a], [ ], [:], [ ], [b]))")
#check($args(a-b: a-b)$, "arguments(a-b: sequence([a], [−], [b]))")
#check($args(a-b)$, "arguments(sequence([a], [−], [b]))")

--- math-call-named-no-expr eval ---
#let func(m: none) = m
// Error: 10 expected expression
$ func(m: ) $

--- math-call-named-duplicate eval ---
#let func(my: none) = my
// Error: 15-17 duplicate argument: my
$ func(my: a, my: b) $

--- math-call-named-shorthand-clash-1 eval ---
#let func(m: none) = m
// Error: 18-21 unexpected argument
$func(m: =) func(m:=)$

--- math-call-named-shorthand-clash-2 eval ---
#let func(m: none) = m
// Error: 41-45 unexpected argument
$func(m::) func(m: :=) func(m:: =) func(m::=)$

--- math-call-named-single-underscore eval ---
#let func(x) = x
// Error: 8-9 expected identifier, found underscore
$ func(_: a) $

--- math-call-named-single-char-error eval ---
#let func(m: none) = m
// Error: 8-13 unexpected argument
$ func(m : a) $

--- math-call-spread-basic eval ---
// Spread arguments in math.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(..#range(0, 4).chunks(2))$, "arguments((0, 1), (2, 3))")
#check($#args(range(1, 5).chunks(2))$, "arguments(((1, 2), (3, 4)))")
#check($#args(..range(1, 5).chunks(2))$, "arguments((1, 2), (3, 4))")
#check($args(#(..range(2, 6).chunks(2)))$, "arguments(((2, 3), (4, 5)))")
#let nums = range(0, 4).chunks(2)
#check($args(..nums)$, "arguments((0, 1), (2, 3))")
#check($args(..nums, ..nums)$, "arguments((0, 1), (2, 3), (0, 1), (2, 3))")
#check($args(..nums, 4, 5)$, "arguments((0, 1), (2, 3), [4], [5])")
#check($args(..nums, ..#range(4, 6))$, "arguments((0, 1), (2, 3), 4, 5)")
#check($args(..nums, #range(4, 6))$, "arguments((0, 1), (2, 3), (4, 5))")

--- math-call-spread-empty eval ---
// Test that a spread operator followed by nothing generates two dots.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(..)$, "arguments(sequence([.], [.]))")
#check($args(.., ..; .. , ..)$, "arguments(\n  (sequence([.], [.]), sequence([.], [.])),\n  (sequence([.], [.]), sequence([.], [.])),\n)")

--- math-call-spread-content-error eval ---
#let args(..body) = body
// Error: 7-16 cannot spread content
$args(..(a + b))$

--- math-call-spread-multiple-exprs eval ---
#let args(..body) = body
// Error: 7-14 cannot spread content
$args(..a + b)$

--- math-call-spread-unexpected-dots eval ---
#let args(..body) = body
// Error: 8-10 unexpected dots
$args(#..range(1, 5).chunks(2))$

--- math-call-spread-unexpected-binary eval ---
// Test spread operators followed by binary math operators with and without
// right operands. These errors aren't great, but they can be silenced with a
// space and no one would actually write this.
$
  // Error: 9-10 unexpected slash
  // Error: 19-20 unexpected hat
  // Error: 29-30 unexpected underscore
  vec(../.) vec(..^.) vec(.._.)
  // Error: 9-10 unexpected slash
  // Error: 19-20 unexpected hat
  // Error: 29-30 unexpected underscore
  vec(../)  vec(..^)  vec(.._)
$

--- math-call-spread-shorthand-clash paged ---
#let func(body) = body
$func(...)$

--- math-call-named-spread-override eval ---
// Test named argument overriding with the spread operator.
#let check(it, s) = test(it.body.text, repr(s))
#let func(a: 1, b: 1) = (a: a, b: b)
#let dict = (a: 2, b: 2)
#let args = arguments(a: 3, b: 3)
#check($func()$, (a: 1, b: 1))
#check($func(..dict, ..args)$, (a: 3, b: 3))
#check($func(..args, ..dict)$, (a: 2, b: 2))
#check($func(a: #4, ..dict, b: #4)$, (a: 2, b: 4))
#check($func(a: #4, ..args, b: #4)$, (a: 3, b: 4))

--- math-call-named-spread-duplicate eval ---
// Test duplicate named args with the spread operator.
// The error should only happen for manually added args.
#let func(..) = none
#let dict = (a: 1)
// Error: 22-23 duplicate argument: a
$func(a: #2, ..dict, a: #3)$

--- math-call-2d-basic eval ---
// Two-dimensional arguments in math. More tests can be found in `mat.typ`.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a;b)$, "arguments(([a],), ([b],))")
#check($args(a,b;c)$, "arguments(([a], [b]), ([c],))")
#check($args(a,b;c,d;e,f)$, "arguments(([a], [b]), ([c], [d]), ([e], [f]))")
#check($args( a; b; )$, "arguments(([a],), ([b],))")
#check($args(a;  ; c)$, "arguments(([a],), ([],), ([c],))")
#check($args(a b,/**/; b)$, "arguments((sequence([a], [ ], [b]), []), ([b],))")
#check($args(a/**/b, ; b)$, "arguments((sequence([a], [b]), []), ([b],))")
#check($args( ;/**/a/**/b/**/; )$, "arguments(([],), (sequence([a], [b]),))")
#check($args( ; , ; )$, "arguments(([],), ([], []))")
#check($args(/**/; // funky whitespace/trivia
    ,   /**/  ;/**/)$, "arguments(([],), ([], []))")

--- math-call-2d-semicolon-embedded-code eval ---
// If a semicolon directly follows an embedded code expression, it terminates
// the code expression instead of indicating 2d arguments.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#check($args(#false;)$, "arguments(false)")
#check($args("a" #"b";)$, "arguments(sequence([a], [ ], [b]))")
#check($args(#true ;)$, "arguments((true,))")
#check($args(#true;;)$, "arguments((true,))")

--- math-call-2d-named eval ---
// Two-dimensional args with named args.
#let args(..body) = (body.pos(), body.named())
#let check(it, r) = test-repr(it.body.text, r)
#check($args(a: b)$, "((), (a: [b]))")
#check($args(a: b,)$, "((), (a: [b]))")
#check($args(a: b;)$, "(((),), (a: [b]))")
#check($args(1, 2; 3, 4)$, "((([1], [2]), ([3], [4])), (:))")

// This set should all be the same.
#check($args(a: b, 1, 2; 3, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, a: b, 2; 3, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, 2, a: b; 3, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, 2; a: b, 3, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, 2; 3, a: b, 4)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, 2; 3, 4, a: b)$, "((([1], [2]), ([3], [4])), (a: [b]))")
#check($args(1, 2; 3, 4; a: b)$, "((([1], [2]), ([3], [4])), (a: [b]))")

#check($args(a: b, 1, 2, 3, c: d)$, "(([1], [2], [3]), (a: [b], c: [d]))")
#check($args(1, 2, 3; a: b)$, "((([1], [2], [3]),), (a: [b]))")
#check($args(a-b: a,, e:f;; d)$, "((([],), ([],), ([d],)), (a-b: [a], e: [f]))")
#check($args(a: b, ..#range(0, 4))$, "((0, 1, 2, 3), (a: [b]))")

--- math-call-2d-spread-pos eval ---
// Two-dimensional args with positional spreading.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#let nums = range(0, 4).chunks(2)
#check($args(..nums;)$, "arguments(((0, 1), (2, 3)))")
#check($args(..nums; ,)$, "arguments(((0, 1), (2, 3)), ([],))")
#check($args(..nums; ;)$, "arguments(((0, 1), (2, 3)), ([],))")
#check($args(..nums; 1, 2; 3, 4)$, "arguments(((0, 1), (2, 3)), ([1], [2]), ([3], [4]))")
#check($args(..nums, 1, 2; 3, 4)$, "arguments(((0, 1), (2, 3), [1], [2]), ([3], [4]))")
#check($args(1, 2; ..nums)$, "arguments(([1], [2]), ((0, 1), (2, 3)))")
#check($args(1, 2; 3, 4)$, "arguments(([1], [2]), ([3], [4]))")
#check($args(1, 2; 3, 4; ..#range(5, 7))$, "arguments(([1], [2]), ([3], [4]), (5, 6))")
#check($args(1, 2; 3, 4, ..#range(5, 7))$, "arguments(([1], [2]), ([3], [4], 5, 6))")
#check($args(1, 2; 3, 4, ..#range(5, 7);)$, "arguments(([1], [2]), ([3], [4], 5, 6))")
#check($args(1, 2; 3, 4, ..#range(5, 7),)$, "arguments(([1], [2]), ([3], [4], 5, 6))")

--- math-call-2d-spread-named eval ---
// Two-dimensional args with named and positional spreading.
#let args(..body) = body
#let check(it, r) = test-repr(it.body.text, r)
#let nums = range(0, 4).chunks(2)
#let dict = (one: 1, two: 2)
#let both = arguments(..nums, ..dict)
#check($args(..nums;)$, "arguments(((0, 1), (2, 3)))")
#check($args(..dict;)$, "arguments(one: 1, two: 2, ())") // Adds an empty array
#check($args(1, ..dict;)$, "arguments(one: 1, two: 2, ([1],))")
#check($args(1, ..dict, 2;)$, "arguments(one: 1, two: 2, ([1], [2]))")
#check($args(1; ..dict, 2;)$, "arguments(one: 1, two: 2, ([1],), ([2],))")
#check($args(1; ..dict; 2;)$, "arguments(one: 1, two: 2, ([1],), (), ([2],))")
#check($args(..nums, ..dict;)$, "arguments(one: 1, two: 2, ((0, 1), (2, 3)))")
#check($args(..both;)$, "arguments(one: 1, two: 2, ((0, 1), (2, 3)))")
#check($args(..nums; ..dict)$, "arguments(one: 1, two: 2, ((0, 1), (2, 3)))")
#check($args(..dict; ..nums)$, "arguments(one: 1, two: 2, (), ((0, 1), (2, 3)))")

--- issue-3774-math-call-empty-2d-args paged ---
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

--- math-call-value-error eval ---
// TODO: We should produce this error for `mat` as well by changing from
// `Value::display()` to `Value::cast::<Content>()`.
$ mat(#1) $
// Error: 8-9 expected content, found integer
$ vec(#1) $

--- math-call-value-non-func eval ---
#test($sin(1)$, $sin(#1)$)

--- issue-2885-math-var-only-in-global eval ---
// Error: 7-10 unknown variable: rgb
// Hint: 7-10 `rgb` is not available directly in math, but is in the standard library
// Hint: 7-10 to access `rgb` in code mode you can add a hash: `#rgb`
// Hint: 7-10 or access `rgb` in math mode by using the `std` module: `std.rgb`
$text(rgb(0, 0, 0), "foo")$

--- math-call-shadowed-builtin paged ---
// We don't error if we try to call a shadowed standard library function.
#let box = "box"
$ box() $

--- math-call-error eval ---
// Test the span of errors when calling a function.
#let func(a, b, c) = {}
// Error: 3-13 missing argument: c
$ func(a, b) $

--- math-call-2d-error eval ---
// Test the span of errors for 2d arguments.
// The current range isn't the best, but it's hard to improve.
#let func() = {}
// Error: 7-20 unexpected argument
$ func(a, b; c, d;) $

--- math-call-error-inside-func eval ---
// Test whether errors inside function calls produce further errors.
#let int = int
$ int(
  // Error: 3-8 missing argument: value
  int()
) $

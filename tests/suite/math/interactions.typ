// Test interactions with styling and normal layout.
// Hint: They are bad ...

--- math-nested-normal-layout ---
// Test images and font fallback.
#let monkey = move(dy: 0.2em, image("/assets/images/monkey.svg", height: 1em))
$ sum_(i=#emoji.apple)^#emoji.apple.red i + monkey/2 $

--- math-table ---
// Test tables.
$ x := #table(columns: 2)[x][y]/mat(1, 2, 3)
     = #table[A][B][C] $

--- math-equation-auto-wrapping ---
// Test non-equation math directly in content.
#math.attach($a$, t: [b])

--- math-font-switch ---
// Test font switch.
// Warning: 29-40 unknown font family: noto sans
#let here = text.with(font: "Noto Sans")
$#here[f] := #here[Hi there]$.

--- math-box-without-baseline ---
// Test boxes without a baseline act as if the baseline is at the base
#{
  box(stroke: 0.2pt, $a #box(stroke: 0.2pt, $a$)$)
  h(12pt)
  box(stroke: 0.2pt, $a #box(stroke: 0.2pt, $g$)$)
  h(12pt)
  box(stroke: 0.2pt, $g #box(stroke: 0.2pt, $g$)$)
}

--- math-box-with-baseline ---
// Test boxes with a baseline are respected
#box(stroke: 0.2pt, $a #box(baseline:0.5em, stroke: 0.2pt, $a$)$)

--- math-at-par-start ---
// Test that equation at start of paragraph works fine.
$x$ is a variable.

--- math-at-par-end ---
// Test that equation at end of paragraph works fine.
One number is $1$

--- math-at-line-start ---
// Test math at the natural end of a line.
#h(60pt) Number $1$ exists.

--- math-at-line-end ---
// Test math at the natural end of a line.
#h(50pt) Number $1$ exists.

--- math-consecutive ---
// Test immediately consecutive equations.
$x$$y$

--- issue-2821-missing-fields ---
// Issue #2821: Setting a figure's supplement to none removes the field
#show figure.caption: it => {
  assert(it.has("supplement"))
  assert(it.supplement == none)
}
#figure([], caption: [], supplement: none)

--- math-symbol-show-rule ---
// Test using rules for symbols
#show sym.tack: it => $#h(1em) it #h(1em)$
$ a tack b $

--- issue-math-realize-show ---
// Test that content in math can be realized without breaking
// nested equations.
#let my = $pi$
#let f1 = box(baseline: 10pt, [f])
#let f2 = context f1
#show math.vec: [nope]

$ pi a $
$ my a $
$ 1 + sqrt(x/2) + sqrt(#hide($x/2$)) $
$ a x #link("url", $+ b$) $
$ f f1 f2 $
$ vec(1,2) * 2 $

--- issue-math-realize-hide ---
$ x^2 #hide[$(>= phi.alt) union y^2 0$] z^2 $
Hello #hide[there $x$]
and #hide[$ f(x) := x^2 $]

--- issue-math-realize-scripting ---
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

--- math-size-resolve ---
#let length = context repr(measure("--").width)
$ a length a ^ length $

--- math-size-arbitrary-content ---
// Test sizing of both relative and absolute non math content in math sizes.
#let stuff = square(inset: 0pt)[hello]
#let square = square(size: 5pt)
$ stuff sum^stuff_square square $

--- math-size-math-content-1 ---
// Nested math content has styles overwritten by the inner equation.
// Ideally the widths would match the actual length of the arrows.
#let arrow = $stretch(->)^"much text"$
$ arrow A^arrow A^A^arrow $
#let width = context measure(arrow).width
$ width A^width A^A^width $

--- math-size-math-content-2 ---
// Nested math content has styles overwritten by the inner equation.
// Ideally the heights would match the actual height of the sums.
#let sum = $sum^2$
#let height(x) = context measure(x).height
$sum = height(sum) $
$ sum != height(sum) $

--- math-text-size ---
// Values retrieved from function are not resolved at the moment.
// Ideally the left size would match the right size.
#let size = context [#text.size.to-absolute() #1em.to-absolute()]
$ size x^size x^x^size $

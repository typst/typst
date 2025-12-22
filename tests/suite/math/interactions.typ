// Test interactions with styling and normal layout.
// Hint: They are bad ...

--- math-nested-normal-layout paged ---
// Test images and font fallback.
#let monkey = move(dy: 0.2em, image("/assets/images/monkey.svg", height: 1em))
$ sum_(i=#emoji.apple)^#emoji.apple.red i + monkey/2 $

--- math-table paged ---
// Test tables.
$ x := #table(columns: 2)[x][y]/mat(1, 2, 3)
     = #table[A][B][C] $

--- math-equation-auto-wrapping paged ---
// Test non-equation math directly in content.
#math.attach($a$, t: [b])

--- math-font-switch paged ---
// Test font switch.
// Warning: 29-40 unknown font family: noto sans
#let here = text.with(font: "Noto Sans")
$#here[f] := #here[Hi there]$.

--- math-root-show-rule-1 paged ---
#show "√": set text(red, font: "Noto Sans Math")
$ root(2, (a + b) / c) $

--- math-root-show-rule-2 paged ---
#show "√": set text(2em)
$ sqrt(2) $

--- math-root-show-rule-3 paged ---
// Test cursed show rule.
#show "√": "!"
$ sqrt(2) root(2, 2) $

--- math-root-show-rule-4 paged ---
#show math.root: set text(red)
$ sqrt(x + y) root(4, 2) $

--- math-root-show-rule-5 paged ---
#show math.root: it => {
  show "√": set text(purple) if it.index == none
  it
}
$ sqrt(1/2) root(3, 1/2) $

--- math-delim-show-rule-1 paged ---
#show regex("\\[|\\]"): set text(green, font: "Noto Sans Math")
$ mat(delim: \[, a, b, c; d, e, f; g, h, i) quad [x + y] $

--- math-delim-show-rule-2 paged ---
#show math.vec: it => {
  show regex("\\(|\\)"): set text(blue)
  it
}
$ vec(1, 0, 0), mat(1; 0; 0), (1), binom(n, k) $

--- math-delim-show-rule-3 paged ---
#show "⏟": set text(fuchsia)
$ underbrace(1 + 1 = 2, "obviously") $

--- math-delim-show-rule-4 paged ---
#show "{": set text(navy)
$ cases(x + y + z = 0, 2x - y = 0, -5y + 2z = 0) $

--- math-delim-show-rule-5 paged ---
#show regex("\\(|\\)"): set text(1.5em)
$ 10 dot (9 - 5) dot (1/2 - 1) $

--- math-primes-show-rule paged ---
#show math.primes: set text(maroon)
$f'(x), f''''''(x)$

--- math-glyph-show-rule paged ---
#show "+": set text(orange, font: "Noto Sans Math")
$ 1 + 1 = +2 $
#show "+": text(2em)[#sym.plus.o]
$ 1 + 1 = +2 $

--- math-accent-show-rule-1 paged ---
#show "\u{0302}": set text(blue, font: "XITS Math")
$hat(x)$, $hat(hat(x))$, x\u{0302}

--- math-accent-show-rule-2 paged ---
#let rhat(x) = {
  show "\u{0302}": set text(red)
  math.hat(x)
}
$hat(x)$, $rhat(x)$, $hat(rhat(x))$, $rhat(hat(x))$, x\u{0302}

--- math-accent-show-rule-3 paged ---
#show math.accent: it => {
  show "\u{0300}": set text(green)
  it
}
$grave(x)$, x\u{0300}

--- math-accent-show-rule-4 paged ---
#show "\u{0302}": box(inset: (bottom: 5pt), text(0.5em, sym.diamond.small))
$hat(X)$, $hat(x)$

--- math-box-without-baseline paged ---
// Test boxes without a baseline act as if the baseline is at the base
#{
  box(stroke: 0.2pt, $a #box(stroke: 0.2pt, $a$)$)
  h(12pt)
  box(stroke: 0.2pt, $a #box(stroke: 0.2pt, $g$)$)
  h(12pt)
  box(stroke: 0.2pt, $g #box(stroke: 0.2pt, $g$)$)
}

--- math-box-with-baseline paged ---
// Test boxes with a baseline are respected
#box(stroke: 0.2pt, $a #box(baseline:0.5em, stroke: 0.2pt, $a$)$)

--- math-at-par-start paged ---
// Test that equation at start of paragraph works fine.
$x$ is a variable.

--- math-at-par-end paged ---
// Test that equation at end of paragraph works fine.
One number is $1$

--- math-at-line-start paged ---
// Test math at the natural end of a line.
#h(60pt) Number $1$ exists.

--- math-at-line-end paged ---
// Test math at the natural end of a line.
#h(50pt) Number $1$ exists.

--- math-consecutive paged ---
// Test immediately consecutive equations.
$x$$y$

--- math-symbol-show-rule paged ---
// Test using rules for symbols
#show sym.tack: it => $#h(1em) it #h(1em)$
$ a tack b $

--- issue-math-realize-show paged ---
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

--- issue-math-realize-hide paged ---
$ x^2 #hide[$(>= phi.alt) union y^2 0$] z^2 $
Hello #hide[there $x$]
and #hide[$ f(x) := x^2 $]

--- issue-math-realize-scripting paged ---
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

--- math-size-resolve paged ---
#let length = context repr(measure("--").width)
$ a length a ^ length $

--- math-size-arbitrary-content paged ---
// Test sizing of both relative and absolute non math content in math sizes.
#let stuff = square(inset: 0pt)[hello]
#let square = square(size: 5pt)
$ stuff sum^stuff_square square $

--- math-size-math-content-1 paged ---
// Nested math content has styles overwritten by the inner equation.
// Ideally the widths would match the actual length of the arrows.
#let arrow = $stretch(->)^"much text"$
$ arrow A^arrow A^A^arrow $
#let width = context measure(arrow).width
$ width A^width A^A^width $

--- math-size-math-content-2 paged ---
// Nested math content has styles overwritten by the inner equation.
// Ideally the heights would match the actual height of the sums.
#let sum = $sum^2$
#let height(x) = context measure(x).height
$sum = height(sum) $
$ sum != height(sum) $

--- math-size-math-content-3 paged ---
// Sum doesn't get wrapped in math as it is a single expr.
// Ideally the height would match the actual height of the sum.
#let height(x) = context measure(x).height
$ sum != height(sum) $

--- math-text-size paged ---
// Values retrieved from function are not resolved at the moment.
// Ideally the left size would match the right size.
#let size = context [#text.size.to-absolute() #1em.to-absolute()]
$ size x^size x^x^size $

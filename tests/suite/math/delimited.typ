// Test delimiter matching and scaling.

--- math-lr-matching ---
// Test automatic matching.
#set page(width:122pt)
$ (a) + {b/2} + abs(a)/2 + (b) $
$f(x/2) < zeta(c^2 + abs(a + b/2))$

--- math-lr-unmatched ---
// Test unmatched.
$[1,2[ = [1,2) != zeta\(x/2\) $

--- math-lr-call ---
// Test manual matching.
$ [|a/b|] != lr(|]a/b|]) != [a/b) $
$ lr(| ]1,2\[ + 1/2|) $

--- math-lr-fences ---
// Test fence confusion.
$ |x + |y| + z/a| \
  lr(|x + |y| + z/a|) $

--- math-lr-symbol-unmatched ---
// Test that symbols aren't matched automatically.
$ bracket.l a/b bracket.r
  = lr(bracket.l a/b bracket.r) $

--- math-lr-half ---
// Test half LRs.
$ lr(a/b\]) = a = lr(\{a/b) $

--- math-lr-size ---
// Test manual scaling.
$ lr(]sum_(x=1)^n x], size: #70%)
  < lr((1, 2), size: #200%) $

--- math-lr-shorthands ---
// Test predefined delimiter pairings.
$floor(x/2), ceil(x/2), abs(x), norm(x)$

--- math-lr-color ---
// Test colored delimiters
$ lr(
    text(\(, fill: #green) a/b
    text(\), fill: #blue)
  ) $

--- math-lr-mid ---
// Test middle functions
$ { x mid(|) sum_(i=1)^oo phi_i (x) < 1 } \
  { integral |dot|
      mid(bar.v.double)
    floor(hat(I) mid(slash) { dot mid(|) dot } mid(|) I/n) } $

--- math-lr-mid-size ---
// Test mid when lr size is set.
#set page(width: auto)

$ lr({ A mid(|) integral }) quad
  lr(size: #1em, { A mid(|) integral }) quad
  lr(size: #(1em+20%), { A mid(|) integral }) \

  lr(] A mid(|) integral ]) quad
  lr(size: #1em, ] A mid(|) integral ]) quad
  lr(size: #(1em+20%), ] A mid(|) integral ]) \

  lr(( A mid(|) integral ]) quad
  lr(size: #1em, ( A mid(|) integral ]) quad
  lr(size: #(1em+20%), ( A mid(|) integral ]) $

--- math-lr-mid-size-nested-equation ---
// Test mid size when lr size is set, when nested in an equation.
#set page(width: auto)

#let body = ${ A mid(|) integral }$
$ lr(body) quad
  lr(size: #1em, body) quad
  lr(size: #(1em+20%), body) $

--- math-lr-unbalanced ---
// Test unbalanced delimiters.
$ 1/(2 (x) $
$ 1_(2 y (x) () $
$ 1/(2 y (x) (2(3)) $

--- math-lr-weak-spacing ---
// Test ignoring weak spacing immediately after the opening
// and immediately before the closing.
$ [#h(1em, weak: true)A(dif x, f(x) dif x)sum#h(1em, weak: true)] $
$ lr(\[#h(1em, weak: true)lr(A dif x, f(x) dif x\))sum#h(1em, weak:true)a) $

--- math-lr-nested ---
// Test nested lr calls.
#let body1 = math.lr($|$, size: 4em)
#let body2 = $lr(|, size: #4em)$

$lr(|, size: #2em)$
$lr(lr(|, size: #4em), size: #50%)$
$lr(body1, size: #50%)$
$lr(body2, size: #50%)$

--- math-lr-ignore-ignorant ---
// Test ignoring leading and trailing ignorant fragments.
#box($ (1 / 2) $)
#box({
  show "(": it => context it
  $ (1 / 2) $
})
#box({
  show ")": it => context it
  $ (1 / 2) $
})
#box({
  show "(": it => context it
  show ")": it => context it
  $ (1 / 2) $
})

--- math-lr-scripts ---
// Test interactions with script attachments.
$ lr(size: #3em, |)_a^b lr(size: #3em, zws|)_a^b
  lr(size: #3em, [x])_0^1 [x]_0^1
  lr(size: #1em, lr(size: #10em, [x]))_0^1 $

--- issue-4188-lr-corner-brackets ---
// Test positioning of U+231C to U+231F
$⌜a⌟⌞b⌝$ = $⌜$$a$$⌟$$⌞$$b$$⌝$

--- math-lr-unparen ---
// Test that unparen with brackets stays as an LrElem.
#let item = $limits(sum)_i$
$
  1 / ([item]) quad
  1 /  [item]
$

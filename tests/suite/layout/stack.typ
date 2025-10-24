// Test stack layouts.

--- stack-basic render ---
// Test stacks with different directions.
#let widths = (
  30pt, 20pt, 40pt, 15pt,
  30pt, 50%, 20pt, 100%,
)

#let shaded(i, w) = {
  let v = (i + 1) * 10%
  rect(width: w, height: 10pt, fill: rgb(v, v, v))
}

#let items = for (i, w) in widths.enumerate() {
  (align(right, shaded(i, w)),)
}

#set page(width: 50pt, margin: 0pt)
#stack(dir: btt, ..items)

--- stack-spacing render ---
// Test spacing.
#set page(width: 50pt, margin: 0pt)

#let x = square(size: 10pt, fill: eastern)
#stack(
  spacing: 5pt,
  stack(dir: rtl, spacing: 5pt, x, x, x),
  stack(dir: ltr, x, 20%, x, 20%, x),
  stack(dir: ltr, spacing: 5pt, x, x, 7pt, 3pt, x),
)

--- stack-overflow render ---
// Test overflow.
#set page(width: 50pt, height: 30pt, margin: 0pt)
#box(stack(
  rect(width: 40pt, height: 20pt, fill: conifer),
  rect(width: 30pt, height: 13pt, fill: forest),
))

--- stack-fr render ---
#set page(height: 3.5cm)
#stack(
  dir: ltr,
  spacing: 1fr,
  ..for c in "ABCDEFGHI" {([#c],)}
)

Hello
#v(2fr)
from #h(1fr) the #h(1fr) wonderful
#v(1fr)
World! 🌍

--- stack-rtl-align-and-fr render ---
// Test aligning things in RTL stack with align function & fr units.
#set page(width: 50pt, margin: 5pt)
#set block(spacing: 5pt)
#set text(8pt)
#stack(dir: rtl, 1fr, [A], 1fr, [B], [C])
#stack(dir: rtl,
  align(center, [A]),
  align(left, [B]),
  [C],
)

--- issue-1240-stack-h-fr render ---
// This issue is sort of horrible: When you write `h(1fr)` in a `stack` instead
// of directly `1fr`, things go awry. To fix this, we now transparently detect
// h/v children.
#stack(dir: ltr, [a], 1fr, [b], 1fr, [c])
#stack(dir: ltr, [a], h(1fr), [b], h(1fr), [c])

--- issue-1240-stack-v-fr render ---
#set page(height: 60pt)
#stack(
  dir: ltr,
  spacing: 1fr,
  stack([a], 1fr, [b]),
  stack([a], v(1fr), [b]),
)

--- issue-1918-stack-with-infinite-spacing render ---
// https://github.com/typst/typst/issues/1918
#set page(width: auto)
#context layout(available => {
  let infinite-length = available.width
  // Error: 3-40 stack spacing is infinite
  stack(spacing: infinite-length)[A][B]
})

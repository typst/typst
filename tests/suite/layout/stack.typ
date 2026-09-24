// Test stack layouts.

--- stack-basic paged ---
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

--- stack-spacing paged ---
// Test spacing.
#set page(width: 50pt, margin: 0pt)

#let x = square(size: 10pt, fill: eastern)
#stack(
  spacing: 5pt,
  stack(dir: rtl, spacing: 5pt, x, x, x),
  stack(dir: ltr, x, 20%, x, 20%, x),
  stack(dir: ltr, spacing: 5pt, x, x, 7pt, 3pt, x),
)

--- stack-overflow paged ---
// Test overflow.
#set page(width: 50pt, height: 30pt, margin: 0pt)
#box(stack(
  rect(width: 40pt, height: 20pt, fill: conifer),
  rect(width: 30pt, height: 13pt, fill: forest),
))

--- issue-8790-stack-overflow-btt paged ---
// https://github.com/typst/typst/issues/8790
// An unbreakable stack keeps each child's height when it overflows.
#set page(width: 160pt, height: 90pt, margin: 10pt)
#set text(size: 9pt)
#let colors = (rgb("#2455c3"), rgb("#168f66"), rgb("#c38a17"), rgb("#b24d72"), rgb("#7b55b6"))
#block(width: 140pt, height: 70pt, breakable: false, clip: true, stroke: 0.5pt)[
  #stack(
    dir: btt,
    ..range(5).map(i => block(
      width: 100%,
      height: 22pt,
      fill: colors.at(i).transparentize(30%),
      text(fill: white, weight: "bold", [item #(i + 1)]),
    )),
  )
]

--- issue-8790-stack-exact-fit paged ---
// An exact fit must keep the bottom-to-top stack inside its region.
#set page(width: 70pt, height: 64pt, margin: 10pt)
#block(width: 50pt, height: 44pt, breakable: false, clip: true)[
  #stack(
    dir: btt,
    block(width: 50pt, height: 22pt, fill: eastern)[A],
    block(width: 50pt, height: 22pt, fill: conifer)[B],
  )
]

--- issue-8790-stack-overflow-visible-btt paged ---
// Expose the overflowing children to make collapsed heights visible.
#set page(width: 70pt, height: 100pt, margin: 10pt)
#block(width: 50pt, height: 44pt, breakable: false, clip: false)[
  #stack(
    dir: btt,
    block(width: 50pt, height: 22pt, fill: eastern)[A],
    block(width: 50pt, height: 22pt, fill: conifer)[B],
    block(width: 50pt, height: 22pt, fill: forest)[C],
  )
]

--- issue-8790-stack-overflow-ttb paged ---
// Top-to-bottom overflow must retain each child's height too.
#set page(width: 70pt, height: 100pt, margin: 10pt)
#block(width: 50pt, height: 44pt, breakable: false, clip: false)[
  #stack(
    dir: ttb,
    block(width: 50pt, height: 22pt, fill: eastern)[A],
    block(width: 50pt, height: 22pt, fill: conifer)[B],
    block(width: 50pt, height: 22pt, fill: forest)[C],
  )
]

--- issue-8790-stack-overflow-spacing paged ---
// Explicit spacing also consumes region height before later children.
#set page(width: 70pt, height: 100pt, margin: 10pt)
#block(width: 50pt, height: 44pt, breakable: false, clip: false)[
  #stack(
    dir: btt,
    spacing: 5pt,
    block(width: 50pt, height: 22pt, fill: eastern)[A],
    block(width: 50pt, height: 22pt, fill: conifer)[B],
    block(width: 50pt, height: 22pt, fill: forest)[C],
  )
]

--- stack-fr paged ---
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

--- stack-rtl-align-and-fr paged ---
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

--- issue-1240-stack-h-fr paged ---
// This issue is sort of horrible: When you write `h(1fr)` in a `stack` instead
// of directly `1fr`, things go awry. To fix this, we now transparently detect
// h/v children.
#stack(dir: ltr, [a], 1fr, [b], 1fr, [c])
#stack(dir: ltr, [a], h(1fr), [b], h(1fr), [c])

--- issue-1240-stack-v-fr paged ---
#set page(height: 60pt)
#stack(
  dir: ltr,
  spacing: 1fr,
  stack([a], 1fr, [b]),
  stack([a], v(1fr), [b]),
)

--- issue-1918-stack-with-infinite-spacing paged ---
// https://github.com/typst/typst/issues/1918
#set page(width: auto)
#context layout(available => {
  let infinite-length = available.width
  // Error: 3-40 stack spacing is infinite
  stack(spacing: infinite-length)[A][B]
})

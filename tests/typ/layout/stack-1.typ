// Test stack layouts.

---
// Test stacks with different directions.
#let widths = (
  30pt, 20pt, 40pt, 15pt,
  30pt, 50%, 20pt, 100%,
)

#let shaded = {
  let v = 0%
  let next() = { v += 10%; rgb(v, v, v) }
  w => rect(width: w, height: 10pt, fill: next())
}

#let items = for w in widths { (align(right, shaded(w)),) }

#set page(width: 50pt, margins: 0pt)
#stack(dir: btt, ..items)

---
// Test spacing.
#set page(width: 50pt, margins: 0pt)

#let x = square(size: 10pt, fill: eastern)
#stack(
  spacing: 5pt,
  stack(dir: rtl, spacing: 5pt, x, x, x),
  stack(dir: ltr, x, 20%, x, 20%, x),
  stack(dir: ltr, spacing: 5pt, x, x, 7pt, 3pt, x),
)

---
// Test overflow.
#set page(width: 50pt, height: 30pt, margins: 0pt)
#box(stack(
  rect(width: 40pt, height: 20pt, fill: conifer),
  rect(width: 30pt, height: 13pt, fill: forest),
))

---
// Test aligning things in RTL stack with align function & fr units.
#set page(width: 50pt, margins: 5pt)
#set text(8pt)
#stack(dir: rtl, 1fr, [A], 1fr, [B], [C])
#v(5pt)
#stack(dir: rtl,
  align(center, [A]),
  align(left, [B]),
  [C],
)

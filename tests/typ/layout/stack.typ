// Test stack layouts.

---
// Test stacks with different directions.
#let widths = (
  30pt, 20pt, 40pt, 15pt,
  30pt, 50%, 20pt, 100%,
)

#let shaded = {
  let v = 0
  let next() = { v += 0.1; rgb(v, v, v) }
  w => rect(width: w, height: 10pt, fill: next())
}

#let items = for w in widths { (shaded(w),) }

#align(right)
#page(width: 50pt, margins: 0pt)
#stack(dir: btt, ..items)
#pagebreak()

// Currently stack works like flexbox apparently.
#stack(dir: ltr, ..items)

---
// Test spacing.
#page(width: 50pt, margins: 0pt)
#par(spacing: 5pt)

#let x = square(length: 10pt, fill: eastern)
#stack(dir: rtl, spacing: 5pt, x, x, x)
#stack(dir: ltr, x, 20%, x, 20%, x)
#stack(dir: ltr, spacing: 5pt, x, x, 7pt, 3pt, x)

---
// Test overflow.
#page(width: 50pt, height: 30pt, margins: 0pt)
#box(stack(
  rect(width: 40pt, height: 20pt, fill: conifer),
  rect(width: 30pt, height: 13pt, fill: forest),
))

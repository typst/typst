// Test show rule patterns.

---
// Inline code.
#show raw.where(block: false): it => box(rect(
  radius: 2pt,
  outset: (y: 3pt),
  inset: (x: 3pt, y: 0pt),
  fill: luma(230),
  it,
))

// Code blocks.
#show raw.where(block: true): rect.with(
  outset: -3pt,
  inset: 11pt,
  fill: luma(230),
  stroke: (left: 1.5pt + luma(180)),
)

#set page(margin: (top: 12pt))
#set par(justify: true)

This code tests `code`
with selectors and justification.

```rs
code!("it");
```

You can use the ```rs *const T``` pointer or
the ```rs &mut T``` reference.

---
#show heading.where(level: 1): set text(red)
#show heading.where(level: 2): set text(blue)
#show heading: set text(green)
= Red
== Blue
=== Green

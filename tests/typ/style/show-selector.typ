// Test show rule patterns.

---
// Inline code.
#show raw.where(block: false): rect.with(
  radius: 2pt,
  outset: (y: 3pt),
  inset: (x: 3pt),
  fill: luma(230),
)

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

---
#show heading.where(level: 1): text.with(red)
#show heading.where(level: 2): text.with(blue)
#show heading: text.with(green)
= Red
== Blue
=== Green

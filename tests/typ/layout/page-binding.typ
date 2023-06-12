// Tests multi-page document with binding.

---
#set page(height: 100pt, margin: (inside: 30pt, outside: 20pt))
#set par(justify: true)
#set text(size: 8pt)

#page(margin: (x: 20pt), {
  set align(center + horizon)
  text(20pt, strong[Title])
  v(2em, weak: true)
  text(15pt)[Author]
})

= Introduction
#lorem(35)

---
// Test setting the binding explicitly.
#set page(margin: (inside: 30pt))
#rect(width: 100%)[Bound]
#pagebreak()
#rect(width: 100%)[Left]

---
// Test setting the binding explicitly.
#set page(binding: right, margin: (inside: 30pt))
#rect(width: 100%)[Bound]
#pagebreak()
#rect(width: 100%)[Right]

---
// Test setting the binding implicitly.
#set page(margin: (inside: 30pt))
#set text(lang: "he")
#rect(width: 100%)[Bound]
#pagebreak()
#rect(width: 100%)[Right]

---
// Error: 19-44 `inside` and `outside` are mutually exclusive with `left` and `right`
#set page(margin: (left: 1cm, outside: 2cm))

---
// Error: 20-23 must be `left` or `right`
#set page(binding: top)

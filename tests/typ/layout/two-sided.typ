// Tests a multi-page document with two-sided: true set for pages

---
#set page(two-sided: true, margin: (outside: 0.25in, inside: 0.5in), height: 100pt)

#set align(center + horizon)

#text(24pt)[Title]
#v(2em, weak: true)
#text(18pt)[Author]
#pagebreak()
#pagebreak()

#set align(left + top)
#set par(justify: true)

= Header 1
#lorem(15)
#pagebreak()

== Header 2
#lorem(30)


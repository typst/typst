// Tests a multi-page document with mixed two-sided and single-sided pages.

---
#set page(margin: (outside: 0.2in, inside: 0.6in), height: 100pt)
== Two-sided
#lorem(15)
#pagebreak()

#set page(margin: (left: 0.3in, right: 0.3in))
== Single-sided
#lorem(15)
#pagebreak()

#set page(margin: (outside: 0.3in, inside: 0.5in))
== Two-sided
#lorem(15)

// Test forced page breaks.

---
First of two
#pagebreak()
#page(height: 40pt)

---
// Make sure that you can't do page related stuff in a container.
A
#box[
  B
  #pagebreak()
  #page("a4")
]
C

// No consequences from the page("A4") call here.
#pagebreak()
D

---
// Test a combination of pages with bodies and normal content.

#page(width: 80pt, height: 30pt)

[#page() First]
[#page() Second]
#pagebreak()
#pagebreak()
Fourth
[#page(height: 25pt)]
Sixth
[#page() Seventh]

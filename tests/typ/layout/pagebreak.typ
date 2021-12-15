// Test forced page breaks.

---
First of two
#pagebreak()
#set page(height: 40pt)
Second of two

---
// Make sure that you can't do page related stuff in a container.
A
#box[
  B
  #pagebreak()
  #set page("a4")
]
C

// No consequences from the page("A4") call here.
#pagebreak()
D

---
// Test a combination of pages with bodies and normal content.

#set page(width: 80pt, height: 30pt)

Fi[#set page(width: 80pt);rst]
[#set page(width: 70pt); Second]
#pagebreak()
#pagebreak()
Fourth
#page(height: 20pt)[]
Sixth
[#set page(); Seventh]

// Test that you can't do page related stuff in a container.

---
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

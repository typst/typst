// Test clearing to even or odd pages.

---
#set page(width: 80pt, height: 30pt)
First
#pagebreak(to: "odd")
Third
#pagebreak(to: "even")
Fourth
#pagebreak(to: "even")
Sixth
#pagebreak()
Seventh
#pagebreak(to: "odd")
#page[Nineth]

---
#set page(width: auto, height: auto)

// Test with auto-sized page.
First
#pagebreak(to: "odd")
Third

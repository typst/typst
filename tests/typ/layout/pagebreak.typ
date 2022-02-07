// Test forced page breaks.

---
// Just a pagebreak.
// Should result in two auto-sized pages.
#pagebreak()

---
// Pagebreak, empty with styles and then pagebreak
// Should result in one auto-sized page and two conifer-colored A11 pages.
#pagebreak()
#set page(width: 2cm, fill: conifer)
#pagebreak()

---
// Test a combination of pagebreaks, styled pages and pages with bodies.
#set page(width: 80pt, height: 30pt)
[#set page(width: 60pt); First]
#pagebreak()
#pagebreak()
Third
#page(height: 20pt, fill: forest)[]
Fif[#set page();th]

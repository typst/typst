// Test forced page breaks.

---
// Just a pagebreak.
// Should result in two pages.
#pagebreak()

---
// Pagebreak, empty with styles and then pagebreak
// Should result in one auto-sized page and two conifer-colored 2cm wide pages.
#pagebreak()
#set page(width: 2cm, fill: conifer)
#pagebreak()

---
// Two text bodies separated with and surrounded by weak pagebreaks.
// Should result in two aqua-colored pages.
#set page(fill: aqua)
#pagebreak(weak: true)
First
#pagebreak(weak: true)
Second
#pagebreak(weak: true)

---
// Test a combination of pagebreaks, styled pages and pages with bodies.
// Should result in three five pages, with the fourth one being forest-colored.
#set page(width: 80pt, height: 30pt)
#[#set page(width: 60pt); First]
#pagebreak()
#pagebreak()
Third
#page(height: 20pt, fill: forest)[]
Fif#[#set page();th]

---
// Test hard and weak pagebreak followed by page with body.
// Should result in three navy-colored pages.
#set page(fill: navy)
#set text(fill: white)
First
#pagebreak()
#page[Second]
#pagebreak(weak: true)
#page[Third]

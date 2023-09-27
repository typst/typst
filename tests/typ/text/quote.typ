// Test the quote element.

---
// Text direction affects author positioning
#let q = quote(author: [René Descartes])[cogito, ergo sum]
And I quote: #q.

#set text(dir: rtl)
And I quote: #q.

---
// Text direction affects block alignment
#set quote(block: true)
#let q = quote(author: [René Descartes])[cogito, ergo sum]

#q

#set text(dir: rtl)
#q

---
// Indentation bar is aligned with text
#set quote(block: true)

#lorem(10)
#quote(lorem(10))
#lorem(10)

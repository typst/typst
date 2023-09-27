// Test the quote element.

---
// Text direction affects author positioning
And I quote: #quote(author: [René Descartes])[cogito, ergo sum].

#set text(dir: rtl)
And I quote: #quote(author: [René Descartes])[cogito, ergo sum].

---
// Text direction affects block alignment
#set quote(block: true)
#let q = quote[
  #quote(author: [René Descartes])[
    cogito, ergo sum
  ]

  A great quote.
]

#q

#set text(dir: rtl)
#q

---
// Indentation bar is aligned with text
#set quote(block: true)

#lorem(10)
#quote(lorem(10))
#lorem(10)

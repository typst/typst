// Test the quote element.

---
// Text direction affects author positioning
And I quote: #quote(author: [René Descartes])[cogito, ergo sum].

#set text(lang: "ar")
#quote(author: [عالم])[مرحبًا]

---
// Text direction affects block alignment
#set quote(block: true)
#quote(author: [René Descartes])[cogito, ergo sum]

#set text(lang: "ar")
#quote(author: [عالم])[مرحبًا]

---
// Indentation bar is aligned with text
#set quote(block: true)

#lorem(10)
#quote(lorem(10))
#lorem(10)

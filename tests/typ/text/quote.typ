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

---
// citation-format: label or numeric
#set quote(block: true, source: <tolkien54>)
#text(0pt, bibliography("/files/works.bib", style: "ieee"))

#quote(author: [Tolkien])[
  In a hole in the ground there lived a hobbit.
]

---
// citation-format: note
#set quote(block: true, source: <tolkien54>)
#text(0pt, bibliography("/files/works.bib", style: "chicago-notes"))

#quote(author: [Tolkien])[
  In a hole in the ground there lived a hobbit.
]

---
// citation-format: author-date or author
#set quote(block: true, source: <tolkien54>)
#text(0pt, bibliography("/files/works.bib", style: "apa"))

#quote(author: [Tolkien])[
  In a hole in the ground there lived a hobbit.
]

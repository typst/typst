// Test the quote element.

---
// Text direction affects author positioning
And I quote: #quote(attribution: [René Descartes])[cogito, ergo sum].

#set text(lang: "ar")
#quote(attribution: [عالم])[مرحبًا]

---
// Text direction affects block alignment
#set quote(block: true)
#quote(attribution: [René Descartes])[cogito, ergo sum]

#set text(lang: "ar")
#quote(attribution: [عالم])[مرحبًا]

---
// Spacing with other blocks
#set quote(block: true)

#lorem(10)
#quote(lorem(10))
#lorem(10)

---
// Inline citation
#bibliography("/files/works.bib")

#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

---
// Citation-format: label or numeric
#set quote(block: true)
#bibliography("/files/works.bib", style: "ieee")

#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

---
// Citation-format: note
#set quote(block: true)
#bibliography("/files/works.bib", style: "chicago-notes")

#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

---
// Citation-format: author-date or author
#set quote(block: true)
#bibliography("/files/works.bib", style: "apa")

#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

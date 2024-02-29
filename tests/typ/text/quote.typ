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
#set text(8pt)

#lorem(10)
#quote(lorem(10))
#lorem(10)

---
// Inline citation
#set text(8pt)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#set text(0pt)
#bibliography("/assets/bib/works.bib")

---
// Citation-format: label or numeric
#set text(8pt)
#set quote(block: true)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#set text(0pt)
#bibliography("/assets/bib/works.bib", style: "ieee")

---
// Citation-format: note
#set text(8pt)
#set quote(block: true)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#set text(0pt)
#bibliography("/assets/bib/works.bib", style: "chicago-notes")

---
// Citation-format: author-date or author
#set text(8pt)
#set quote(block: true)
#quote(attribution: <tolkien54>)[In a hole in the ground there lived a hobbit.]

#set text(0pt)
#bibliography("/assets/bib/works.bib", style: "apa")

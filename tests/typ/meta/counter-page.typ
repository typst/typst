// Test the page counter.

#set page(height: 50pt, margin: (bottom: 20pt, rest: 10pt))
#lorem(12)
#set page(numbering: "(i)")
#lorem(6)
#pagebreak()
#set page(numbering: "1 / 1")
#counter(page).update(1)
#lorem(20)

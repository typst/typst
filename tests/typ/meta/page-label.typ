#set page(margin: (bottom: 20pt, rest: 10pt))
#let filler = lorem(20)

// (i) - (ii). No style opt. because of suffix.
#set page(numbering: "(i)")
#filler
#pagebreak()
#filler

// 3 - 4. Style opt. Page Label should use /D style.
#set page(numbering: "1")
#filler
#pagebreak()
#filler

// I - IV. Style opt. Page Label should use /R style and start at 1 again.
#set page(numbering: "I / I")
#counter(page).update(1)
#filler
#pagebreak()
#filler
#pagebreak()
#filler
#pagebreak()
#filler

// Pre: ほ, Pre: ろ, Pre: は, Pre: に. No style opt. Uses prefix field entirely.
// Counter update without numbering change.
#set page(numbering: "Pre: い")
#filler
#pagebreak()
#filler
#counter(page).update(2)
#filler
#pagebreak()
#filler
#pagebreak()
#filler

// aa & ba. Style opt only for values <= 26. Page Label uses lower alphabet style.
// Repeats letter each 26 pages or uses numbering directly as prefix.
#set page(numbering: "a")
#counter(page).update(27)
#filler
#pagebreak()
#counter(page).update(53)
#filler

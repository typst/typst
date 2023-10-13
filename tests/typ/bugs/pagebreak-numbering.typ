// https://github.com/typst/typst/issues/2095
// The empty page 2 should not have a page number

#set page(numbering: none)
This and next page should not be numbered

#pagebreak(weak: true, to: "odd")

#set page(numbering: "1")
#counter(page).update(1)

This page should


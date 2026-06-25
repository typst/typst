--- format-options-single-page-range paged empty ---
#set pdf(pages: 3)
#set pdf(pages: (from: 3))
#set pdf(pages: (to: 5))
#set pdf(pages: (from: 3, to: 5))

--- format-options-multiple-page-ranges paged empty ---
#set pdf(pages: (5, 7, 9))
#set pdf(pages: ((to: 3), 5, (from: 7, to: 8), (from: 10)))

--- format-options-page-nr-string eval ---
// Error: 17-20 expected integer, dictionary, array, or none, found string
#set pdf(pages: "3")

--- format-options-page-range-str eval ---
// Error: 17-22 expected integer, dictionary, array, or none, found string
#set pdf(pages: "3-5")

--- format-options-page-nr-zero eval ---
// Error: 17-18 page numbers start at one
#set pdf(pages: 0)

--- format-options-page-range-inverted eval ---
// Error: 17-33 page export range start cannot be larger than end
#set pdf(pages: (from: 5, to: 2))

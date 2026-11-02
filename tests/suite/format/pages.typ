--- format-options-single-page-range paged empty ---
#set pdf(tagged: false, pages: 3)
#set pdf(tagged: false, pages: (from: 3))
#set pdf(tagged: false, pages: (to: 5))
#set pdf(tagged: false, pages: (from: 3, to: 5))

--- format-options-multiple-page-ranges paged empty ---
#set pdf(tagged: false, pages: (5, 7, 9))
#set pdf(tagged: false, pages: ((to: 3), 5, (from: 7, to: 8), (from: 10)))

--- format-options-page-range-untagged-warnings pdf empty ---
// Warning: exporting a page range disables PDF tagging
// Hint: the resulting PDF will be inaccessible
// Hint: set `pdf(tagged: false)` to silence this warning
#set pdf(pages: 1)

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
// Error: 17-33 start of page range cannot be larger than end
#set pdf(pages: (from: 5, to: 2))

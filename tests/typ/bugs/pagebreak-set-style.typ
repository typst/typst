// https://github.com/typst/typst/issues/2162
// The styles should not be applied to the pagebreak empty page,
// it should only be applied after that.

#pagebreak(to: "even") // We should now skip to page 2

Some text on page 2

#pagebreak(to: "even") // We should now skip to page 4

#set page(fill: orange) // This sets the color of the page starting from page 4
Some text on page 4

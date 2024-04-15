// Test forced page breaks.

--- pagebreak ---
// Just a pagebreak.
// Should result in two pages.
#pagebreak()

--- pagebreak-around-set-page ---
// Pagebreak, empty with styles and then pagebreak
// Should result in one auto-sized page and two conifer-colored 2cm wide pages.
#pagebreak()
#set page(width: 2cm, fill: conifer)
#pagebreak()

--- pagebreak-weak-after-set-page ---
// Two text bodies separated with and surrounded by weak pagebreaks.
// Should result in two aqua-colored pages.
#set page(fill: aqua)
#pagebreak(weak: true)
First
#pagebreak(weak: true)
Second
#pagebreak(weak: true)

--- pagebreak-set-page-mixed ---
// Test a combination of pagebreaks, styled pages and pages with bodies.
// Should result in three five pages, with the fourth one being forest-colored.
#set page(width: 80pt, height: 30pt)
#[#set page(width: 60pt); First]
#pagebreak()
#pagebreak()
Third
#page(height: 20pt, fill: forest)[]
Fif#[#set page();th]

--- pagebreak-followed-by-page-call ---
// Test hard and weak pagebreak followed by page with body.
// Should result in three navy-colored pages.
#set page(fill: navy)
#set text(fill: white)
First
#pagebreak()
#page[Second]
#pagebreak(weak: true)
#page[Third]

--- pagebreak-in-container ---
#box[
  // Error: 4-15 pagebreaks are not allowed inside of containers
  #pagebreak()
]

--- pagebreak-weak-place ---
// After place
// Should result in three pages.
First
#pagebreak(weak: true)
#place(right)[placed A]
#pagebreak(weak: true)
Third

--- pagebreak-weak-meta ---
// After only ignorables & invisibles
// Should result in two pages.
First
#pagebreak(weak: true)
#counter(page).update(1)
#metadata("Some")
#pagebreak(weak: true)
Second

--- pagebreak-meta ---
// After only ignorables, but regular break
// Should result in three pages.
First
#pagebreak()
#counter(page).update(1)
#metadata("Some")
#pagebreak()
Third

--- pagebreak-to ---
#set page(width: 80pt, height: 30pt)
First
#pagebreak(to: "odd")
Third
#pagebreak(to: "even")
Fourth
#pagebreak(to: "even")
Sixth
#pagebreak()
Seventh
#pagebreak(to: "odd")
#page[Ninth]

--- pagebreak-to-auto-sized ---
#set page(width: auto, height: auto)

// Test with auto-sized page.
First
#pagebreak(to: "odd")
Third

--- pagebreak-to-multiple-pages ---
#set page(height: 30pt, width: 80pt)

// Test when content extends to more than one page
First

Second

#pagebreak(to: "odd")

Third

--- issue-2134-pagebreak-bibliography ---
// Test weak pagebreak before bibliography.
#pagebreak(weak: true)
#bibliography("/assets/bib/works.bib")

--- issue-2095-pagebreak-numbering ---
// The empty page 2 should not have a page number
#set page(numbering: none)
This and next page should not be numbered

#pagebreak(weak: true, to: "odd")

#set page(numbering: "1")
#counter(page).update(1)

This page should

--- issue-2162-pagebreak-set-style ---
// The styles should not be applied to the pagebreak empty page,
// it should only be applied after that.
#pagebreak(to: "even") // We should now skip to page 2

Some text on page 2

#pagebreak(to: "even") // We should now skip to page 4

#set page(fill: orange) // This sets the color of the page starting from page 4
Some text on page 4

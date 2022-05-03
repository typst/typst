// Test the page class.

---
// Just empty page.
// Should result in auto-sized page, just like nothing.
#page[]

---
// Just empty page with styles.
// Should result in one conifer-colored A11 page.
#page("a11", flipped: true, fill: conifer)[]

---
// Set width and height.
// Should result in one high and one wide page.
#set page(width: 80pt, height: 80pt)
[#set page(width: 40pt);High]
[#set page(height: 40pt);Wide]

// Flipped predefined paper.
[#set page(paper: "a11", flipped: true);Flipped A11]

---
// Test page fill.
#set page(width: 80pt, height: 40pt, fill: eastern)
#text(15pt, "Roboto", fill: white, smallcaps: true)[Typst]
#page(width: 40pt, fill: none, margins: (top: 10pt, rest: auto))[Hi]

---
// Just page followed by pagebreak.
// Should result in one forest-colored A11 page and one auto-sized page.
#page("a11", flipped: true, fill: forest)[]
#pagebreak()

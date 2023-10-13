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
#[#set page(width: 40pt);High]
#[#set page(height: 40pt);Wide]

// Flipped predefined paper.
#[#set page(paper: "a11", flipped: true);Flipped A11]

---
// Test page fill.
#set page(width: 80pt, height: 40pt, fill: eastern)
#text(15pt, font: "Roboto", fill: white, smallcaps[Typst])
#page(width: 40pt, fill: none, margin: (top: 10pt, rest: auto))[Hi]

---
// Just page followed by pagebreak.
// Should result in one forest-colored A11 page and one auto-sized page.
#page("a11", flipped: true, fill: forest)[]
#pagebreak()

---
// Layout without any container should provide the page's dimensions, minus its margins.

#page(width: 100pt, height: 100pt, {
  layout(size => [This page has a width of #size.width and height of #size.height ])
  h(1em)
  place(left, rect(width: 80pt, stroke: blue))
})

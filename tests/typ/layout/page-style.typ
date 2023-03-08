// Test setting page styles.

---
// Empty with styles
// Should result in one conifer-colored A11 page.
#set page("a11", flipped: true, fill: conifer)

---
// Empty with styles and then pagebreak
// Should result in two forest-colored pages.
#set page(fill: forest)
#pagebreak()

---
// Empty with multiple page styles.
// Should result in a small white page.
#set page("a4")
#set page("a5")
#set page(width: 1cm, height: 1cm)

---
// Empty with multiple page styles.
// Should result in one eastern-colored A11 page.
#set page("a4")
#set page("a5")
#set page("a11", flipped: true, fill: eastern)
#set text(font: "Roboto", white)
#smallcaps[Typst]

// Test hyperlinking.

---
// Link syntax.
https://example.com/

// Link with body.
#link("https://typst.app/")[Some text text text]

// With line break.
This link appears #link("https://google.com/")[in the middle of] a paragraph.

// Certain prefixes are trimmed when using the `link` function.
Contact #link("mailto:hi@typst.app") or
call #link("tel:123") for more information.

---
// Test that the period is trimmed.
https://a.b.?q=%10#. \
Wahttp://link \
Nohttps:\//link \
Nohttp\://comment

---
// Styled with underline and color.
#set link(fill: rgb("283663"))
You could also make the
#link("https://html5zombo.com/")[link look way more typical.]

---
// Transformed link.
#set page(height: 60pt)
#set link(underline: false)
#let mylink = link("https://typst.app/")[LINK]
My cool #move(dx: 0.7cm, dy: 0.7cm, rotate(10deg, scale(200%, mylink)))

---
// Link containing a block.
#link("https://example.com/", underline: false, block[
  My cool rhino
  #move(dx: 10pt, image("/res/rhino.png", width: 1cm))
])

---
// Link to page one.
#link((page: 1, x: 10pt, y: 20pt))[Back to the start]

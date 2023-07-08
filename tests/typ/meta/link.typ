// Test hyperlinking.

---
// Link syntax.
https://example.com/

// Link with body.
#link("https://typst.org/")[Some text text text]

// With line break.
This link appears #link("https://google.com/")[in the middle of] a paragraph.

// Certain prefixes are trimmed when using the `link` function.
Contact #link("mailto:hi@typst.app") or
call #link("tel:123") for more information.

---
// Test that the period is trimmed.
#show link: underline
https://a.b.?q=%10#. \
Wahttp://link \
Nohttps:\//link \
Nohttp\://comment

---
// Verify that brackets are included in links.
https://[::1]:8080/ \
https://example.com/(paren) \
https://example.com/#(((nested))) \

---
// Check that unbalanced brackets are not included in links.
#[https://example.com/] \
https://example.com/)

---
// Verify that opening brackets without closing brackets throw an error.
// Error: 1-22 automatic links cannot contain unbalanced brackets, use the `link` function instead
https://exam(ple.com/

---
// Styled with underline and color.
#show link: it => underline(text(fill: rgb("283663"), it))
You could also make the
#link("https://html5zombo.com/")[link look way more typical.]

---
// Transformed link.
#set page(height: 60pt)
#let mylink = link("https://typst.org/")[LINK]
My cool #box(move(dx: 0.7cm, dy: 0.7cm, rotate(10deg, scale(200%, mylink))))

---
// Link containing a block.
#link("https://example.com/", block[
  My cool rhino
  #box(move(dx: 10pt, image("/files/rhino.png", width: 1cm)))
])

---
// Link to page one.
#link((page: 1, x: 10pt, y: 20pt))[Back to the start]

---
// Test link to label.
Text <hey>
#link(<hey>)[Go to text.]

---
// Error: 2-20 label does not exist in the document
#link(<hey>)[Nope.]

---
Text <hey>
Text <hey>
// Error: 2-20 label occurs multiple times in the document
#link(<hey>)[Nope.]

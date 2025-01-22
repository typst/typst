// Test hyperlinking.

--- link-basic render html ---
// Link syntax.
https://example.com/

// Link with body.
#link("https://typst.org/")[Some text text text]

// With line break.
This link appears #link("https://google.com/")[in the middle of] a paragraph.

// Certain prefixes are trimmed when using the `link` function.
Contact #link("mailto:hi@typst.app") or
call #link("tel:123") for more information.

--- link-trailing-period ---
// Test that the period is trimmed.
#show link: underline
https://a.b.?q=%10#. \
Wahttp://link \
Nohttps:\//link \
Nohttp\://comment

--- link-bracket-balanced ---
// Verify that brackets are included in links.
https://[::1]:8080/ \
https://example.com/(paren) \
https://example.com/#(((nested))) \

--- link-bracket-unbalanced-closing ---
// Check that unbalanced brackets are not included in links.
#[https://example.com/] \
https://example.com/)

--- link-bracket-unbalanced-opening ---
// Verify that opening brackets without closing brackets throw an error.
// Error: 1-22 automatic links cannot contain unbalanced brackets, use the `link` function instead
https://exam(ple.com/

--- link-show ---
// Styled with underline and color.
#show link: it => underline(text(fill: rgb("283663"), it))
You could also make the
#link("https://html5zombo.com/")[link look way more typical.]

--- link-transformed ---
// Transformed link.
#set page(height: 60pt)
#let mylink = link("https://typst.org/")[LINK]
My cool #box(move(dx: 0.7cm, dy: 0.7cm, rotate(10deg, scale(200%, mylink))))

--- link-on-block ---
// Link containing a block.
#link("https://example.com/", block[
  My cool rhino
  #box(move(dx: 10pt, image("/assets/images/rhino.png", width: 1cm)))
])

--- link-to-page ---
// Link to page one.
#link((page: 1, x: 10pt, y: 20pt))[Back to the start]

--- link-to-label ---
// Test link to label.
Text <hey>
#link(<hey>)[Go to text.]

--- link-to-label-missing ---
// Error: 2-20 label `<hey>` does not exist in the document
#link(<hey>)[Nope.]

--- link-to-label-duplicate ---
Text <hey>
Text <hey>
// Error: 2-20 label `<hey>` occurs multiple times in the document
#link(<hey>)[Nope.]

--- link-empty-block ---
#link("", block(height: 10pt, width: 100%))

--- issue-758-link-repeat ---
#let url = "https://typst.org/"
#let body = [Hello #box(width: 1fr, repeat[.])]

Inline: #link(url, body)

#link(url, block(inset: 4pt, [Block: ] + body))

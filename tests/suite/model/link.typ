// Test hyperlinking.

--- link-basic render html pdftags ---
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

--- link-html-id-attach html ---
// Tests how IDs and, if necessary, spans, are added to the DOM to support
// links.

#for i in range(1, 10) {
  list.item(link(label("t" + str(i)), [Go]))
}

// Text at start of paragraph
Hi <t1>

// Text at start of paragraph + more text
Hi <t2> there

// Text in the middle of paragraph
See #[it <t4>]

// Text in the middle of paragraph + more text
See #[it <t5>] here

// Text + more elements
See #[a *b*] <t6>

// Element
See *a _b_* <t3>

// Nothing
See #[] <t7>

// Nothing 2
See #metadata(none) <t8>

*Strong* <t9>

--- link-html-label-disambiguation html ---
// Tests automatic ID generation for labelled elements.

#[= A] #label("%") // not reusable => loc-1
= B <1>            // not reusable => loc-3 (loc-2 exists)
= C <loc>          // reusable, unique => loc
= D <loc-2>        // reusable, unique => loc-2
= E <lib>          // reusable, not unique => lib-1
= F <lib>          // reusable, not unique => lib-3 (lib-2 exists)
= G <lib-2>        // reusable, unique => lib-2
= H <hi>           // reusable, unique => hi
= I <hi-2>         // reusable, not unique => hi-2-1
= J <hi-2>         // reusable, not unique => hi-2-2

#context for it in query(heading) {
  list.item(link(it.location(), it.body))
}

--- link-html-id-existing html ---
// Test that linking reuses the existing ID, if any.
#html.div[
  #html.span(id: "this")[This] <other>
]

#link(<other>)[Go]

--- link-html-here html ---
#context link(here())[Go]

--- link-html-nested-empty html ---
#[#metadata(none) <a> #metadata(none) <b> Hi] <c>

#link(<a>)[A] // creates empty span
#link(<b>)[B] // creates second empty span
#link(<c>)[C] // links to #a because the generated span is contained in it

--- link-html-frame html ---
= Frame 1
#html.frame(block(
  stroke: 1pt,
  width: 200pt,
  height: 500pt,
)[
  #place(center, dy: 100pt, stack(
    dir: ltr,
    spacing: 10pt,
    link(<f1>, square(size: 10pt, fill: teal)),
    link(<text>, square(size: 10pt, fill: black)),
    link(<f2>, square(size: 10pt, fill: yellow)),
  ))
  #place(center, dy: 200pt)[
    #square(size: 10pt, fill: teal) <f1>
  ]
])

= Text <text>
#link(<f1>)[Go to teal square]

= Frame 2
#html.frame(block(
  stroke: 1pt,
  width: 200pt,
  height: 500pt,
)[
  #place(center, dy: 100pt)[
    #square(size: 10pt, fill: yellow) <f2>
  ]
])

--- link-html-frame-ref html ---
// Test that reference links work in `html.frame`. Currently, references (and a
// few other elements) do not internally use `LinkElem`s, so they trigger a
// slightly different code path; see `typst-html/src/link.rs`. The text show
// rule is only there to keep the output small.
#set heading(numbering: "1")
#show "Section" + sym.space.nobreak + "1": rect()
#html.frame[@intro]
= Introduction <intro>

--- link-to-label-missing ---
// Error: 2-20 label `<hey>` does not exist in the document
#link(<hey>)[Nope.]

--- link-to-label-duplicate ---
Text <hey>
Text <hey>
// Error: 2-20 label `<hey>` occurs multiple times in the document
#link(<hey>)[Nope.]

--- link-empty-url ---
// Error: 7-9 URL must not be empty
#link("")[Empty]

--- link-empty-block ---
#link("https://example.com", block(height: 10pt, width: 100%))

--- issue-758-link-repeat ---
#let url = "https://typst.org/"
#let body = [Hello #box(width: 1fr, repeat[.])]

Inline: #link(url, body)

#link(url, block(inset: 4pt, [Block: ] + body))

--- locate-position render ---
// Test `locate`.
#v(10pt)
= Introduction <intro>
#context test(locate(<intro>).position().y, 20pt)

--- locate-position-trailing-tag render ---
// Test locating the position of a tag with no following content.
#context test(here().position().y, 10pt)
#box[]
#v(10pt)
#context test(here().position().y, 20pt)

--- locate-start-of-par render ---
#metadata(none)<a>A#metadata(none)<b>B

// The first metadata has its end tag before the paragraph, so it does not
// become part of the paragraph and thus its Y position is determined by the
// flow.
#context assert(
  locate(<a>).position().y < locate(<b>).position().y
)

// The first footnote becomes part of the paragraph. Thus, its Y position is
// determined by inline layout.
#footnote[c]<c>C#footnote[d]<d>D

#context test(
  locate(<c>).position().y,
  locate(<d>).position().y,
)

--- locate-missing-label render ---
// Error: 10-25 label `<intro>` does not exist in the document
#context locate(<intro>)

--- locate-duplicate-label render ---
= Introduction <intro>
= Introduction <intro>

// Error: 10-25 label `<intro>` occurs multiple times in the document
#context locate(<intro>)

--- locate-element-selector render ---
#v(10pt)
= Introduction <intro>
#context test(locate(heading).position().y, 20pt)

--- locate-element-selector-no-match render ---
// Error: 10-25 selector does not match any element
#context locate(heading)

--- locate-element-selector-multiple-matches render ---
= Introduction <intro>
= Introduction <intro>

// Error: 10-25 selector matches multiple elements
#context locate(heading)

--- locate-between-pages render ---
// Test locating tags that are before or between pages.
#set page(height: 30pt)
#context [
  // Before the first page.
  // (= at the very start of the first page, before the header)
  #test(locate(<a>).position(), (page: 1, x: 0pt, y: 0pt))

  // On the first page.
  #test(locate(<b>).position(), (page: 1, x: 10pt, y: 10pt))

  // Between the two pages.
  // (= at the very start of the first page, before the header)
  #test(locate(<c>).position(), (page: 2, x: 0pt, y: 0pt))

  // After the last page.
  // (= at the very end of the last page, after the footer)
  #test(locate(<d>).position(), (page: 2, x: 0pt, y: 30pt))
  #test(locate(<e>).position(), (page: 2, x: 0pt, y: 30pt))
]

#metadata(none) <a>
#pagebreak(weak: true)
#metadata(none) <b>
A
#pagebreak()
#metadata(none) <c>
#pagebreak(weak: true)
B
#pagebreak(weak: true)
#metadata(none) <d>
#pagebreak(weak: true)
#metadata(none) <e>

--- locate-migrated-breakable render ---
// Ensure that when a breakable element fully migrates to the next page without
// orphan frames, its position correctly reflects that.
#set page(height: 40pt)
A
#block[B]<b>

#context test(
  locate(<b>).position(),
  (page: 2, x: 10pt, y: 10pt),
)

--- issue-4029-locate-after-spacing render ---
#set page(margin: 10pt)
#show heading: it => v(40pt) + it

= Introduction
#context test(
  locate(heading).position(),
  (page: 1, x: 10pt, y: 50pt),
)


--- issue-4029-locate-after-pagebreak render ---
#set page(margin: 10pt)
#show heading: it => pagebreak() + it

= Introduction
#context test(
  locate(heading).position(),
  (page: 2, x: 10pt, y: 10pt),
)

--- issue-4029-locate-after-par-and-pagebreak render ---
// Ensure that the heading's tag isn't stuck at the end of the paragraph.
#set page(margin: 10pt)
Par
#show heading: it => pagebreak() + it
= Introduction
#context test(locate(heading).page(), 2)

--- issue-1886-locate-after-metadata render ---
#show heading: it => {
  metadata(it.label)
  pagebreak(weak: true, to: "odd")
  it
}

Hi
= Hello <hello>
= World <world>

// The metadata's position does not migrate to the next page, but the heading's
// does.
#context {
  test(locate(metadata.where(value: <hello>)).page(), 1)
  test(locate(<hello>).page(), 3)
  test(locate(metadata.where(value: <world>)).page(), 3)
  test(locate(<world>).page(), 5)
}

--- issue-1833-locate-place render ---
#set page(height: 60pt)
#context {
  place(right + bottom, rect())
  test(here().position(), (page: 1, x: 10pt, y: 10pt))
}

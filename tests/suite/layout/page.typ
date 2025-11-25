// Test the page class.

--- page-call-empty ---
// Just empty page.
// Should result in auto-sized page, just like nothing.
#page[]

--- page-call-styled-empty ---
// Just empty page with styles.
// Should result in one conifer-colored A11 page.
#page("a11", flipped: true, fill: conifer)[]

--- page-call-followed-by-pagebreak ---
// Just page followed by pagebreak.
// Should result in one forest-colored A11 page and one auto-sized page.
#page("a11", flipped: true, fill: forest)[]
#pagebreak()

--- page-set-forces-break ---
// Set width and height.
// Should result in one high and one wide page.
#set page(width: 80pt, height: 80pt)
#[#set page(width: 40pt);High]
#[#set page(height: 40pt);Wide]

// Flipped predefined paper.
#[#set page(paper: "a11", flipped: true);Flipped A11]

--- page-set-in-container ---
#box[
  // Error: 4-18 page configuration is not allowed inside of containers
  #set page("a4")
]

--- page-set-empty ---
// Empty with styles
// Should result in one conifer-colored A11 page.
#set page("a11", flipped: true, fill: conifer)

--- page-set-only-pagebreak ---
// Empty with styles and then pagebreak
// Should result in two forest-colored pages.
#set page(fill: forest)
#pagebreak()

--- page-set-override-thrice ---
// Empty with multiple page styles.
// Should result in a small white page.
#set page("a4")
#set page("a5")
#set page(width: 1cm, height: 1cm)

--- page-set-override-and-mix ---
// Empty with multiple page styles.
// Should result in one eastern-colored A11 page.
#set page("a4")
#set page("a5")
#set page("a11", flipped: true, fill: eastern)
#set text(font: "Roboto", white)
#smallcaps[Typst]

--- page-large ---
#set page("a4")

--- page-fill ---
// Test page fill.
#set page(width: 80pt, height: 40pt, fill: eastern)
#text(15pt, font: "Roboto", fill: white, smallcaps[Typst])
#page(width: 40pt, fill: auto, margin: (top: 10pt, rest: auto))[Hi]

--- page-fill-none ---
// Test disabling page fill.
// The PNG is filled with black anyway due to the test runner.
#set page(fill: none)
#rect(fill: green)

--- page-margin-uniform ---
// Set all margins at once.
#[
  #set page(height: 20pt, margin: 5pt)
  #place(top + left)[TL]
  #place(bottom + right)[BR]
]

--- page-margin-individual ---
// Set individual margins.
#set page(height: 40pt)
#[#set page(margin: (left: 0pt)); #align(left)[Left]]
#[#set page(margin: (right: 0pt)); #align(right)[Right]]
#[#set page(margin: (top: 0pt)); #align(top)[Top]]
#[#set page(margin: (bottom: 0pt)); #align(bottom)[Bottom]]

// Ensure that specific margins override general margins.
#[#set page(margin: (rest: 0pt, left: 20pt)); Overridden]

--- page-margin-inside-outside-override ---
#set page(height: 100pt, margin: (inside: 30pt, outside: 20pt))
#set par(justify: true)
#set text(size: 8pt)

#page(margin: (x: 20pt), {
  set align(center + horizon)
  text(20pt, strong[Title])
  v(2em, weak: true)
  text(15pt)[Author]
})

= Introduction
#lorem(35)

--- page-margin-inside ---
#set page(margin: (inside: 30pt))
#rect(width: 100%)[Bound]
#pagebreak()
#rect(width: 100%)[Left]

--- page-margin-inside-with-binding ---
// Test setting the binding explicitly.
#set page(binding: right, margin: (inside: 30pt))
#rect(width: 100%)[Bound]
#pagebreak()
#rect(width: 100%)[Right]

--- page-margin-binding-from-text-lang ---
// Test setting the binding implicitly.
#set page(margin: (inside: 30pt))
#set text(lang: "he")
#rect(width: 100%)[Bound]
#pagebreak()
#rect(width: 100%)[Right]

--- page-margin-left-and-outside ---
// Error: 19-44 `inside` and `outside` are mutually exclusive with `left` and `right`
#set page(margin: (left: 1cm, outside: 2cm))

--- page-margin-binding-bad ---
// Error: 20-23 must be `left` or `right`
#set page(binding: top)

--- page-marginals ---
#set page(
  paper: "a8",
  margin: (x: 15pt, y: 30pt),
  header: {
    text(eastern)[*Typst*]
    h(1fr)
    text(0.8em)[_Chapter 1_]
  },
  footer: context align(center)[\~ #counter(page).display() \~],
  background: context if counter(page).get().first() <= 2 {
    place(center + horizon, circle(radius: 1cm, fill: luma(90%)))
  }
)

#align(center, lines(20))

#set page(header: none, height: auto, margin: (top: 15pt, bottom: 25pt))
Z

--- page-number-align-top-right ---
#set page(
  height: 100pt,
  margin: 30pt,
  numbering: "(1)",
  number-align: top + right,
)

#block(width: 100%, height: 100%, fill: aqua.lighten(50%))

--- page-number-align-bottom-left ---
#set page(
  height: 100pt,
  margin: 30pt,
  numbering: "[1]",
  number-align: bottom + left,
)

#block(width: 100%, height: 100%, fill: aqua.lighten(50%))

--- page-number-align-left-horizon ---
// Error: 25-39 expected `top` or `bottom`, found horizon
#set page(number-align: left + horizon)

--- page-numbering-pdf-label ---
#set page(margin: (bottom: 20pt, rest: 10pt))
#let filler = lines(7)

// (i) - (ii). No style opt. because of suffix.
#set page(numbering: "(i)")
#filler
#pagebreak()
#filler

// 3 - 4. Style opt. Page Label should use /D style.
#set page(numbering: "1")
#filler
#pagebreak()
#filler

// I - IV. Style opt. Page Label should use /R style and start at 1 again.
#set page(numbering: "I / I")
#counter(page).update(1)
#filler
#pagebreak()
#filler
#pagebreak()
#filler
#pagebreak()
#filler

// Pre: ほ, Pre: ろ, Pre: は, Pre: に. No style opt. Uses prefix field entirely.
// Counter update without numbering change.
#set page(numbering: "Pre: い")
#filler
#pagebreak()
#filler
#counter(page).update(2)
#filler
#pagebreak()
#filler
#pagebreak()
#filler

// aa & ba. Style opt only for values <= 26. Page Label uses lower alphabet style.
// Repeats letter each 26 pages or uses numbering directly as prefix.
#set page(numbering: "a")
#counter(page).update(27)
#filler
#pagebreak()
#counter(page).update(53)
#filler

--- page-numbering-hint ---
= Heading <intro>

// Error: 1:21-1:47 cannot reference without page numbering
// Hint: 1:21-1:47 you can enable page numbering with `#set page(numbering: "1")`
Can not be used as #ref(<intro>, form: "page")

--- page-suppress-headers-and-footers ---
#set page(header: none, footer: none, numbering: "1")
Look, ma, no page numbers!

#pagebreak()

#set page(header: auto, footer: auto)
Default page numbers now.

--- page-numbering-huge ---
#set page(margin: (bottom: 20pt, rest: 0pt))
#let filler = lines(1)

// Test values greater than 32-bits
#set page(numbering: "1/1")
#counter(page).update(100000000001)
#pagebreak()
#pagebreak()

--- page-marginal-style-text-set ---
#set page(numbering: "1", margin: (bottom: 20pt))
#set text(red)
Red

--- page-marginal-style-text-set-first ---
#set text(red)
#set page(numbering: "1", margin: (bottom: 20pt))
Red

--- page-marginal-style-text-call ---
#set page(numbering: "1", margin: (bottom: 20pt))
#text(red)[Red]

--- page-marginal-style-text-call-code ---
#{
  set page(numbering: "1", margin: (bottom: 20pt))
  text(red)[Red]
}

--- page-marginal-style-text-call-around-page-call ---
#text(red, page(numbering: "1", margin: (bottom: 20pt))[Hello])

--- page-marginal-style-text-call-around-set-page ---
#text(red, {
  set page(numbering: "1", margin: (bottom: 20pt))
  text(style: "italic")[Hello]
})

--- page-marginal-style-text-call-around-pagebreak ---
#set page(numbering: "1", margin: (bottom: 20pt))
A
#text(red)[
  #pagebreak(weak: true)
  B
]

--- page-marginal-style-show-rule ---
#set page(numbering: "1", margin: (bottom: 20pt))
= Introduction

--- page-marginal-style-show-rule-with-set-page ---
#show heading: it => {
  set page(numbering: "1", margin: (bottom: 20pt))
  it
}

= Introduction

--- page-marginal-style-show-rule-with-page-call ---
#show heading: page.with(fill: aqua)

A
= Introduction
B

--- page-marginal-style-show-rule-with-pagebreak ---
#set page(numbering: "1", margin: (bottom: 20pt))
#show heading: it => {
  pagebreak(weak: true)
  it
}

= Introduction

--- page-marginal-style-context ---
#set page(numbering: "1", margin: (bottom: 20pt))
#show: it => context {
  set text(red)
  it
}
Hi

--- page-marginal-style-shared-initial-interaction ---
#set page(numbering: "1", margin: (bottom: 20pt))
A
#{
  set text(fill: red)
  pagebreak()
}
#text(fill: blue)[B]

--- page-marginal-style-empty ---
#set text(red)
#set page(numbering: "1", margin: (bottom: 20pt))

--- page-marginal-style-page-call ---
#page(numbering: "1", margin: (bottom: 20pt))[
  #set text(red)
  A
]

--- issue-2631-page-header-ordering ---
#set text(6pt)
#show heading: set text(6pt, weight: "regular")
#set page(
  margin: (x: 10pt, top: 20pt, bottom: 10pt),
  height: 50pt,
  header: context {
    let prev = query(selector(heading).before(here()))
    let next = query(selector(heading).after(here()))
    let prev = if prev != () { prev.last().body }
    let next = if next != () { next.first().body }
    (prev: prev, next: next)
  }
)

= First
Hi
#pagebreak()
= Second

--- issue-4340-set-document-and-page ---
// Test custom page fields being applied on the last page
// if the document has custom fields.
#set document(author: "")
#set page(fill: gray)
text
#pagebreak()

--- issue-2326-context-set-page ---
#context [
  #set page(fill: aqua)
  On page #here().page()
]

--- issue-3671-get-from-page-call ---
#set page(margin: 5pt)
#context test(page.margin, 5pt)
#page(margin: 10pt, context test(page.margin, 10pt))

--- issue-4363-set-page-after-tag ---
#set page(fill: aqua)
1
#pagebreak()
#metadata(none)
#set page(fill: red)
2

--- issue-7292-page-width-auto-margin-zero ---
#set page(width: auto, height: 100pt, margin: 0pt)

--- issue-7292-page-height-auto-margin-zero ---
#set page(width: 100pt, height: auto, margin: 0pt)

--- issue-7292-page-size-auto-margin-zero ---
#set page(width: auto, height: auto, margin: 0pt)

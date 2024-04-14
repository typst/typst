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
#page(width: 40pt, fill: none, margin: (top: 10pt, rest: auto))[Hi]

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

But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun. Arise, fair sun, and kill the envious moon, Who is already sick and
pale with grief, That thou her maid art far more fair than she: Be not her maid,
since she is envious; Her vestal livery is but sick and green And none but fools
do wear it; cast it off. It is my lady, O, it is my love! O, that she knew she
were! She speaks yet she says nothing: what of that? Her eye discourses; I will
answer it.

#set page(header: none, height: auto, margin: (top: 15pt, bottom: 25pt))
The END.

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
#let filler = lorem(20)

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

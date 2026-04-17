// Test micro-typographical shenanigans.

--- overhang paged ---
// Test hanging punctuation.
// TODO: This test was broken at some point.
#set page(width: 130pt, margin: 15pt)
#set par(justify: true, linebreaks: "simple")
#set text(size: 9pt)
#rect(inset: 0pt, fill: rgb(0, 0, 0, 0), width: 100%)[
  This is a little bit of text that builds up to
  hang-ing hyphens and dash---es and then, you know,
  some punctuation in the margin.
]

// Test hanging punctuation with RTL.
#set text(lang: "he", font: ("PT Sans", "Noto Serif Hebrew"))
בנייה נכונה של משפטים ארוכים דורשת ידע בשפה. אז בואו נדבר על מזג האוויר.

--- overhang-label paged ---
// Test that labels do not interfere with hanging punctuation.
#set page(width: 130pt)
#set par(justify: true)
#set text(size: 11pt)
#block(stroke: 0.5pt + blue, width: 4cm)[This is a text with just, the right length.]
#block(stroke: 0.5pt + blue, width: 4cm)[This is a text with just,<label> the right length.]

--- overhang-dash paged ---
#set page(margin: 2pt)
#set par(justify: true)
#set text(size: 11pt)
#block(stroke: 0.5pt + blue, width: 25pt, { h(1fr); sym.wj; sym.dash.em })
#block(stroke: 0.5pt + blue, width: 25pt, { h(1fr); sym.dash.em })
#set text(dir: rtl)
#block(stroke: 0.5pt + blue, width: 25pt, { h(1fr); sym.dash.em })
#block(stroke: 0.5pt + blue, width: 25pt, { h(1fr); sym.wj; sym.dash.em; })
#set text(dir: ltr)
#show "—": sym.wj + sym.dash.em
#block(stroke: 0.5pt + blue, width: 25pt, { h(1fr); sym.dash.em })

--- issue-8130-overhang-in-show-rule paged ---
#set page(margin: 2pt)
#set par(justify: true)
#set text(size: 11pt)
#let f(body) = {
  show "x": "y"
  body
}
#block(stroke: 0.5pt + blue, width: 10pt, [#h(1fr);#f[a,] b])
#block(stroke: 0.5pt + blue, width: 10pt, [#h(1fr);a, b])

--- overhang-lone paged ---
// Test that lone punctuation doesn't overhang into the margin.
#set page(margin: 0pt)
#set align(end)
#set text(dir: rtl)
:

--- overhang-start paged ---
// Test hanging punctuation at the start of a line.
#set page(width: 130pt, margin: 15pt)
#set text(overhang: start)
And then he said: #linebreak(justify: true)

— Hello, world!

#set text(lang: "he", font: ("PT Sans", "Noto Serif Hebrew"))
— שלום, עולם!

--- overhang-left paged ---
// Test hanging punctuation at the left margin.
#set page(width: 130pt, margin: 15pt)
#set text(overhang: left)
And then he said: #linebreak(justify: true)

— Hello, world!

#set text(lang: "he", font: ("PT Sans", "Noto Serif Hebrew"))
— שלום, עולם!

--- overhang-right paged ---
// Test hanging punctuation at the right margin.
#set page(width: 130pt, margin: 15pt)
#set text(overhang: right)
And then he said: #linebreak(justify: true)

— Hello, world!

#set text(lang: "he", font: ("PT Sans", "Noto Serif Hebrew"))
— שלום, עולם!

--- overhang-custom paged ---
// Test custom overhang values.
#set page(width: 130pt, margin: 15pt)
#let protrusion-table = (
  "”": (100%, 100%),
  "“": (100%, 100%),
  "A": (10%, 0%),
)
#set text(
  size: 9pt,
  overhang: (map: protrusion-table, default: right),
)
#set par(justify: true, linebreaks: "simple")
“A well-proportioned overhang can make a paragraph look much more
visually appealing,” #linebreak(justify: true)
she said.

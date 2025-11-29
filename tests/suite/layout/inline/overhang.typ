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

--- overhang-lone paged ---
// Test that lone punctuation doesn't overhang into the margin.
#set page(margin: 0pt)
#set align(end)
#set text(dir: rtl)
:

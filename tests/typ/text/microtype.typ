// Test micro-typographical shenanigans.

---
// Test hanging punctuation.
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

---
// Test that lone punctuation doesn't overhang into the margin.
#set page(margin: 0pt)
#set align(end)
#set text(dir: rtl)
:

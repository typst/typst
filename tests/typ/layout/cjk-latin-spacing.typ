// Test CJK-Latin spacing.

#set page(width: 50pt + 10pt, margin: (x: 5pt))
#set text(lang: "zh", font: "Noto Serif CJK SC", cjk-latin-spacing: auto)
#set par(justify: true)

中文，中12文1中，文12中文

中文，中ab文a中，文ab中文

#set text(cjk-latin-spacing: none)

中文，中12文1中，文12中文

中文，中ab文a中，文ab中文

---
// Issue #2538
#set text(cjk-latin-spacing: auto)

abc字

abc字#linebreak()

abc字#linebreak()
母

abc字\
母

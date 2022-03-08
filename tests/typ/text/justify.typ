
---
#set page(width: 180pt)
#set par(
  lang: "en",
  justify: true,
  indent: 14pt,
  spacing: 0pt,
  leading: 5pt,
)

This text is justified, meaning that spaces are stretched so that the text
forms a "block" with flush edges at both sides.

First line indents and hyphenation play nicely with justified text.

---
// Test that lines with hard breaks aren't justified.
#set par(justify: true)
A B C \
D

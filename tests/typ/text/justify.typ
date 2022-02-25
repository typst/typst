
---
#set par(indent: 14pt, spacing: 0pt, leading: 5pt, justify: true)

This text is justified, meaning that spaces are stretched so that the text
forms as "block" with flush edges at both sides.

First line indents and hyphenation play nicely with justified text.

---
// Test that lines with hard breaks aren't justified.
#set par(justify: true)
A B C \
D

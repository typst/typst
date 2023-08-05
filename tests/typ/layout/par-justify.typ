
---
#set page(width: 180pt)
#set block(spacing: 5pt)
#set par(justify: true, first-line-indent: 14pt, leading: 5pt)

This text is justified, meaning that spaces are stretched so that the text
forms a "block" with flush edges at both sides.

First line indents and hyphenation play nicely with justified text.

---
// Test that lines with hard breaks aren't justified.
#set par(justify: true)
A B C \
D

---
// Test forced justification with justified break.
A B C #linebreak(justify: true)
D E F #linebreak(justify: true)

---
// Test that there are no hick-ups with justification enabled and
// basically empty paragraph.
#set par(justify: true)
#""

---
// Test that the last line can be shrunk
#set page(width: 155pt)
#set par(justify: true)
This text can be fitted in one line.

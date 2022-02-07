// Test that set affects the instantiation site and not the
// definition site of a template.

---
// Test that text is affected by instantiation-site bold.
#let x = [World]
Hello *{x}*

---
// Test that lists are affected by correct indents.
#let fruit = [
  - Apple
  - Orange
  #list(body-indent: 10pt, [Pear])
]

- Fruit
[#set list(label-indent: 10pt)
 #fruit]
- No more fruit

---
// Test that that par spacing and text style are respected from
// the outside, but the more specific fill is respected.
#set par(spacing: 0pt)
#set text(style: "italic", fill: eastern)
#let x = [And the forest #parbreak() lay silent!]
#text(fill: forest, x)

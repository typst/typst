// Test SVG with text.

---
#set page(width: 250pt)

#figure(
  image("/assets/images/diagram.svg"),
  caption: [A textful diagram],
)

---
#set page(width: 250pt)
#show image: set text(font: ("Roboto", "Noto Serif CJK SC"))

#figure(
  image("/assets/images/chinese.svg"),
  caption: [Bilingual text]
)

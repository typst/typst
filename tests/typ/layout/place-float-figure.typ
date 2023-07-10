// Test floating figures.

---
#set page(height: 250pt, width: 150pt)

= Introduction
#lorem(10) #footnote[Lots of Latin]

#figure(
  placement: bottom,
  caption: [A glacier #footnote[Lots of Ice]],
  image("/files/glacier.jpg", width: 80%),
)

#lorem(40)

#figure(
  placement: top,
  caption: [An important],
  image("/files/diagram.svg", width: 80%),
)

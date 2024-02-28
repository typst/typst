// Test floating figures.

---
#set page(height: 250pt, width: 150pt)

= Introduction
#lorem(10) #footnote[Lots of Latin]

#figure(
  placement: bottom,
  caption: [A glacier #footnote[Lots of Ice]],
  image("/assets/images/glacier.jpg", width: 80%),
)

#lorem(40)

#figure(
  placement: top,
  caption: [An important],
  image("/assets/images/diagram.svg", width: 80%),
)

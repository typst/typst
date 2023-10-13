// Test a list of figures.

---
#set page(
  paper: "a7",
  numbering: "1 / 1",
  margin: (bottom: 1cm, rest: 0.5cm),
)

#set figure(numbering: "I")
#show figure: set image(width: 80%)

= List of Figures
#locate(it => {
  let elements = query(selector(figure).after(it), it)
  for it in elements [
    Figure
    #numbering(it.numbering,
      ..counter(figure).at(it.location())):
    #it.caption.body
    #box(width: 1fr, repeat[.])
    #counter(page).at(it.location()).first() \
  ]
})

#figure(
  image("/files/glacier.jpg"),
  caption: [Glacier melting],
)

#figure(
  rect[Just some stand-in text],
  kind: image,
  supplement: "Figure",
  caption: [Stand-in text],
)

#figure(
  image("/files/tiger.jpg"),
  caption: [Tiger world],
)

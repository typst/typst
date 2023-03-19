// Test the query function.

---
#set page(
  paper: "a7",
  margin: (y: 1cm, x: 0.5cm),
  header: {
    smallcaps[Typst Academy]
    h(1fr)
    locate(it => {
      let after = query(heading, after: it)
      let before = query(heading, before: it)
      let elem = if before.len() != 0 {
        before.last()
      } else if after.len() != 0 {
        after.first()
      }
      emph(elem.body)
    })
  }
)

#outline()

= Introduction
#lorem(35)

= Background
#lorem(35)

= Approach
#lorem(60)

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
  let elements = query(figure, after: it)
  for it in elements [
    Figure
    #numbering(it.numbering,
      ..counter(figure).at(it.location())):
    #it.caption
    #box(width: 1fr, repeat[.])
    #counter(page).at(it.location()).first() \
  ]
})

#figure(
  image("/glacier.jpg"),
  caption: [Glacier melting],
)

#figure(
  rect[Just some stand-in text],
  caption: [Stand-in text],
)

#figure(
  image("/tiger.jpg"),
  caption: [Tiger world],
)

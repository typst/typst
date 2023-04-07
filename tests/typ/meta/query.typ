// Test the query function.

---
#set page(
  paper: "a7",
  margin: (y: 1cm, x: 0.5cm),
  header: {
    smallcaps[Typst Academy]
    h(1fr)
    locate(it => {
      let after = query(selector(heading).after(it), it)
      let before = query(selector(heading).before(it), it)
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
  let elements = query(selector(figure).after(it), it)
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
  kind: image,
  supplement: "Figure",
  caption: [Stand-in text],
)

#figure(
  image("/tiger.jpg"),
  caption: [Tiger world],
)

---
#set page(
  paper: "a7",
  numbering: "1 / 1",
  margin: (bottom: 1cm, rest: 0.5cm),
)

#show heading.where(level: 1, outlined: true): it => [
  #it

  #set text(size: 12pt, weight: "regular")
  #outline(
      title: "Chapter outline", 
      indent: true,
      target: heading.where(level: 1).or(heading.where(level: 2))
        .after(it.location(), inclusive: true)
        .before(heading.where(level: 1, outlined: true).after(it.location(), inclusive: false), inclusive: false))
]

#set heading(outlined: true, numbering: "1.")

= Section 1
#lorem(30)
== Subsection 1
#lorem(30)
== Subsection 2
#lorem(30)
=== Subsubsection 1
#lorem(30)
=== Subsubsection 2
#lorem(30)
== Subsection 3
#lorem(30)

#pagebreak()

= Section 2
#lorem(30)
== Subsection 1
#lorem(30)
== Subsection 2
#lorem(30)

#pagebreak()

= Section 3

#lorem(30)
== Subsection 1
#lorem(30)
== Subsection 2
#lorem(30)
=== Subsubsection 1
#lorem(30)
=== Subsubsection 2
#lorem(30)
=== Subsubsection 3
#lorem(30)
== Subsection 3
#lorem(30)

--- 

#set page(
  paper: "a7",
  numbering: "1 / 1",
  margin: (bottom: 1cm, rest: 0.5cm),
)

#set heading(outlined: true, numbering: "1.")

// This is purposefully an empty 
#locate(loc => [
  Non-outlined elements: #query(selector(heading).and(heading.where(outlined: false)), loc).map(it => it.body).join(", ")
])

#heading("A", outlined: false)
#heading("B", outlined: true)
#heading("C", outlined: true)
#heading("D", outlined: false)
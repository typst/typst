// Test citation in other introspection.

---
#set page(width: 180pt)
#set heading(numbering: "1")

#outline(
  title: [List of Figures],
  target: figure.where(kind: image),
)

#pagebreak()

= Introduction <intro>
#figure(
  rect[-- PIRATE --],
  caption: [A pirate @arrgh in @intro],
)

#context [Citation @distress on page #here().page()]

#pagebreak()
#bibliography("/assets/bib/works.bib", style: "chicago-notes")

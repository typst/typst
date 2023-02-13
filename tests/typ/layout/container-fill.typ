#set page(height: 100pt)
#let words = lorem(18).split()
#block(inset: 8pt, fill: aqua, stroke: aqua.darken(30%))[
  #words.slice(0, 12).join(" ")
  #box(fill: teal, outset: 2pt)[incididunt]
  #words.slice(12).join(" ")
]

#page("a8")
#place(bottom, center)[© Typst]

= Placement
#place(right, image("../../res/tiger.jpg", width: 1.8cm))
Hi there. This is \
a placed node. \
Unfortunately, \
the line breaks still had to be inserted manually.

#stack(
  rect(fill: eastern, height: 10pt),
  place(right, dy: 1.5pt)[ABC],
  rect(fill: conifer, height: 10pt, width: 80%),
  rect(fill: forest, height: 10pt),
)

#block[
  #place(center, dx: -7pt, dy: -5pt)[Hello]
  #place(center, dx: 7pt, dy: 5pt)[Hello]
  Hello #h(1fr) Hello
]

// Test the `place` function.

---
#set page("a8")
#place(bottom + center)[© Typst]

= Placement
#place(right, image("/files/tiger.jpg", width: 1.8cm))
Hi there. This is \
a placed element. \
Unfortunately, \
the line breaks still had to be inserted manually.

#stack(
  rect(fill: eastern, height: 10pt, width: 100%),
  place(right, dy: 1.5pt)[ABC],
  rect(fill: conifer, height: 10pt, width: 80%),
  rect(fill: forest, height: 10pt, width: 100%),
  10pt,
  block[
    #place(center, dx: -7pt, dy: -5pt)[Hello]
    #place(center, dx: 7pt, dy: 5pt)[Hello]
    Hello #h(1fr) Hello
  ]
)

---
// Test how the placed element interacts with paragraph spacing around it.
#set page("a8", height: 60pt)

First

#place(bottom + right)[Placed]

Second

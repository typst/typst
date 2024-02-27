// Test the `hide` function.

---
AB #h(1fr) CD \
#hide[A]B #h(1fr) C#hide[D]
---
Hidden:
#hide[#line(length: 100%)]
#line(length: 100%)
---
Hidden:
#hide(table(rows: 2, columns: 2)[a][b][c][d])
#table(rows: 2, columns: 2)[a][b][c][d]
---
Hidden:
#hide[
  #polygon((20%, 0pt),
    (60%, 0pt),
    (80%, 2cm),
    (0%,  2cm),)
]
#polygon((20%, 0pt),
  (60%, 0pt),
  (80%, 2cm),
  (0%,  2cm),)
---
#set rect(
  inset: 8pt,
  fill: rgb("e4e5ea"),
  width: 100%,
)

Hidden:
#hide[
#grid(
  columns: (1fr, 1fr, 2fr),
  rows: (auto, 40pt),
  gutter: 3pt,
  rect[A],
  rect[B],
  rect[C],
  rect(height: 100%)[D],
)
]
#grid(
  columns: (1fr, 1fr, 2fr),
  rows: (auto, 40pt),
  gutter: 3pt,
  rect[A],
  rect[B],
  rect[C],
  rect(height: 100%)[D],
)
---

Hidden:
#hide[
- 1
- 2
  1. A
  2. B
- 3
]


- 1
- 2
  1. A
  2. B
- 3

---
Hidden:
#hide(image("/assets/images/tiger.jpg", width: 5cm, height: 1cm,))

#image("/assets/images/tiger.jpg", width: 5cm, height: 1cm,)

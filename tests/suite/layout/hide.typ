// Test the `hide` function.

--- hide-text ---
AB #h(1fr) CD \
#hide[A]B #h(1fr) C#hide[D]

--- hide-line ---
Hidden:
#hide[#line(length: 100%)]
#line(length: 100%)

--- hide-table ---
Hidden:
#hide(table(rows: 2, columns: 2)[a][b][c][d])
#table(rows: 2, columns: 2)[a][b][c][d]

--- hide-polygon ---
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

--- hide-rect ---
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

--- hide-list ---
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

--- hide-image ---
Hidden:
#hide(image("/assets/images/tiger.jpg", width: 5cm, height: 1cm,))

#image("/assets/images/tiger.jpg", width: 5cm, height: 1cm,)

--- issue-622-hide-meta-cite ---
// Test that metadata of hidden stuff stays available.
#set cite(style: "chicago-notes")

A pirate. @arrgh \
#set text(2pt)
#hide[
  A @arrgh pirate.
  #bibliography("/assets/bib/works.bib")
]

--- issue-622-hide-meta-outline ---
#set text(8pt)
#outline()
#set text(2pt)
#hide(block(grid(
  [= A],
  [= B],
  block(grid(
    [= C],
    [= D],
  ))
)))

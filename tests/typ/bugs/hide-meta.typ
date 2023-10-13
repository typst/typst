// Test that metadata of hidden stuff stays available.

---
#set cite(style: "chicago-notes")

A pirate. @arrgh \
#set text(2pt)
#hide[
  A @arrgh pirate.
  #bibliography("/files/works.bib")
]

---
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

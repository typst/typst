// Test class construction.

---
// Ensure that constructor styles aren't passed down the tree.
// The inner list should have no extra indent.
#set par(leading: 2pt)
#list(
  body-indent: 20pt,
  [First],
  list([A], [B])
)

---
// Ensure that constructor styles win, but not over outer styles.
// The outer paragraph should be right-aligned,
// but the B should be center-aligned.
#set par(align: center)
#par(align: right)[
  A #rect(width: 2cm, fill: conifer, padding: 4pt)[B]
]

---
// The inner rectangle should also be yellow here.
// (and therefore invisible)
[#set rect(fill: yellow);#text(100%, rect(padding: 5pt, rect()))]

---
// The inner rectangle should not be yellow here.
A #rect(fill: yellow, padding: 5pt, rect()) B

---
// The inner list should not be indented extra.
[#set text(100%);#list(indent: 20pt, list[A])]

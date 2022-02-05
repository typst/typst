// Test class construction.

---
// Ensure that constructor styles aren't passed down the tree.
#set par(spacing: 2pt)
#list(
  body-indent: 20pt,
  [First],
  list([A], [B])
)

---
// Ensure that constructor styles win, but not over outer styles.
#set par(align: center)
#par(align: right)[
  A #rect(width: 2cm, fill: conifer, padding: 4pt)[B]
]

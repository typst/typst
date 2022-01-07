// Test class construction.

---
// Ensure that constructor styles aren't passed down the tree.
#set par(spacing: 2pt)
#list(
  body-indent: 20pt,
  [First],
  list([A], [B])
)

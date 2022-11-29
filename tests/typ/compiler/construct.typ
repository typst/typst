// Test node functions.

---
// Ensure that constructor styles aren't passed down the tree.
// The inner list should have no extra indent.
#set par(leading: 2pt)
#list(body-indent: 20pt, [First], list[A][B])

---
// Ensure that constructor styles win, but not over outer styles.
// The outer paragraph should be right-aligned,
// but the B should be center-aligned.
#set par(align: center)
#par(align: right)[
  A #rect(width: 2cm, fill: conifer, inset: 4pt)[B]
]

---
// The inner rectangle should also be yellow here.
// (and therefore invisible)
[#set rect(fill: yellow);#text(1em, rect(inset: 5pt, rect()))]

---
// The inner rectangle should not be yellow here.
A #rect(fill: yellow, inset: 5pt, rect()) B

---
// The constructor property should still work
// when there are recursive show rules.
#show list: set text(blue)
#list(label: "(a)", [A], list[B])

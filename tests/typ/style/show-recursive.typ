// Test recursive show rules.

---
// Test basic identity.
#show it: heading as it
= Heading

---
// Test more recipes down the chain.
#show it: list as scale(origin: left, x: 80%, it)
#show heading as []
#show enum as []
- Actual
- Tight
- List

---
// Test recursive base recipe. (Burn it with fire!)
#set list(label: [- Hey])
- Labelless
- List

---
// Test show rule in function.
#let starwars(body) = [
  #show v: list as {
    stack(dir: ltr,
      text(red, v),
      1fr,
      scale(x: -100%, text(blue, v)),
    )
  }
  #body
]

- Normal list
#starwars[
  - Star
  - Wars
  - List
]
- Normal list

---
// Test multi-recursion with nested lists.
#set rect(padding: 2pt)
#show v: list as rect(stroke: blue, v)
#show v: list as rect(stroke: red, v)

- List
  - Nested
  - List
- Recursive!

---
// Inner heading is not finalized. Bug?
#set heading(around: none)
#show it: heading as it.body
#show heading as [
  = A [
    = B
  ]
]

= Discarded

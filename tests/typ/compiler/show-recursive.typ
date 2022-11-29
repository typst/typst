// Test recursive show rules.

---
// Test basic identity.
#show heading: it => it
= Heading

---
// Test more recipes down the chain.
#show list: scale.with(origin: left, x: 80%)
#show heading: []
#show enum: []
- Actual
- Tight
- List
= Nope

---
// Test show rule in function.
#let starwars(body) = {
  show list: it => block({
    stack(dir: ltr,
      text(red, it),
      1fr,
      scale(x: -100%, text(blue, it)),
    )
  })
  body
}

- Normal list

#starwars[
  - Star
  - Wars
  - List
]

- Normal list

---
// Test multi-recursion with nested lists.
#set rect(inset: 3pt)
#show list: rect.with(stroke: blue)
#show list: rect.with(stroke: red)
#show list: block

- List
  - Nested
  - List
- Recursive!

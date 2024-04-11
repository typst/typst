// Stacks with infinite spacing were causing a panic.
// https://github.com/typst/typst/issues/3917
// Ref: false

#set page(width: auto)
#context layout(available => {
  let infinite-length = available.width
  // Error: 3-40 stack spacing is infinite
  stack(spacing: infinite-length)[A][B]
})

---
#set page(width: auto)
#context layout(available => {
  let infinite-length = available.width
  // Error: 3-50 cannot create grid with infinite width
  grid(gutter: infinite-length, columns: 2)[A][B]
})

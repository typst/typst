// Test field access.
// Ref: false

---
// Test field on dictionary.
#let dict = (nothing: "ness", hello: "world")
#test(dict.nothing, "ness")
{
  let world = dict
    .hello

  test(world, "world")
}

---
// Test field on node.
#show node: list as {
  test(node.items.len(), 3)
}

- A
- B
- C

---
// Error: 6-13 dictionary does not contain key "invalid"
{(:).invalid}

---
// Error: 2-7 cannot access field on boolean
{false.ok}

---
// Error: 29-32 unknown field "fun"
#show node: heading as node.fun
= A

---
// Error: 8-12 expected identifier, found boolean
{false.true}

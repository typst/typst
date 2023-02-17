// Test field access.
// Ref: false

---
// Test field on dictionary.
#let dict = (nothing: "ness", hello: "world")
#test(dict.nothing, "ness")
#{
  let world = dict
    .hello

  test(world, "world")
}

---
// Test field on node.
#show list: node => {
  test(node.items.len(), 3)
}

- A
- B
- C

---
// Error: 6-13 dictionary does not contain key "invalid"
#(:).invalid

---
// Error: 8-10 cannot access fields on type boolean
#false.ok

---
// Error: 29-32 unknown field `fun`
#show heading: node => node.fun
= A

---
// Error: 9 expected identifier
// Error: 9 expected semicolon or line break
#{false.true}

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
// Test fields on elements.
#show list: it => {
  test(it.children.len(), 3)
}

- A
- B
- C

---
// Test fields on function scopes.
#enum.item
#assert.eq
#assert.ne

---
// Error: 9-16 function `assert` does not contain field `invalid`
#assert.invalid

---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid

---
// Error: 7-14 function `enum` does not contain field `invalid`
#enum.invalid()

---
// Closures cannot have fields.
#let f(x) = x
// Error: 4-11 cannot access fields on user-defined functions
#f.invalid

---
// Error: 6-13 dictionary does not contain key "invalid" and no default value was specified
#(:).invalid

---
// Error: 8-10 cannot access fields on type boolean
#false.ok

---
// Error: 25-28 content does not contain field "fun" and no default value was specified
#show heading: it => it.fun
= A

---
// Error: 9-13 cannot access fields on type boolean
#{false.true}

// Test field access.
// Ref: false

---
#let dict = (nothing: "ness", hello: "world")
#test(dict.nothing, "ness")
{
  let world = dict
    .hello

  test(world, "world")
}

---
// Error: 6-13 dictionary does not contain key "invalid"
{(:).invalid}

---
// Error: 2-7 cannot access field on boolean
{false.ok}

---
// Error: 8-12 expected identifier, found boolean
{false.true}

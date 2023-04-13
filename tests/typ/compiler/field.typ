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

---
// Test relative length fields.
#test((100% + 2em + 2pt).relative, 100%)
#test((100% + 2em + 2pt).fixed, 2em + 2pt)

---
// Test length fields.
#test((1pt).em, 0em)
#test((1pt).absolute, 1pt)
#test((3em).em, 3em)
#test((3em).absolute, 0pt)
#test((2em + 2pt).em, 2em)
#test((2em + 2pt).absolute, 2pt)

---
// Test length unit conversions.
#test((2em + 2.345pt).pt, 2.345)
#test((2em).pt, 0)
#test((3.345cm).cm, 3.345)
#test((4.345mm).mm, 4.345)
#test((5.345in).inches, 5.345)

---
// Test color fields.
#test(rgb(1, 2, 3, 4).rgba, (1, 2, 3, 4))
#test(rgb(1, 2, 3).rgba, (1, 2, 3, 255))
#test(rgb(1, 2, 3).hex, "#010203")
#test(rgb(1, 2, 3, 4).hex, "#01020304")
#test(repr(cmyk(4%, 5%, 6%, 7%).cmyk), repr((3.9%, 5.1%, 5.9%, 7.1%)))
#test(luma(40).luma, 40)

---
// Test stroke fields.
#test((1em + blue).thickness, 1em)
#test((1em + blue).color, blue)

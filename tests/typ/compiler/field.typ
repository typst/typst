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
#test((100% + 2em + 2pt).absolute, 2em + 2pt)
#test((100% + 2pt).absolute, 2pt)
#test((100% + 2pt - 2pt).absolute, 0pt)
#test((56% + 2pt - 56%).relative, 0%)

---
// Test length fields.
#test((1pt).em, 0em)
#test((1pt).pt, 1pt)
#test((3em).em, 3em)
#test((3em).pt, 0pt)
#test((2em + 2pt).em, 2em)
#test((2em + 2pt).pt, 2pt)

---
// Test color fields.
#test(rgb(1, 2, 3, 4).values, (1, 2, 3, 4))
#test(rgb(1, 2, 3).values, (1, 2, 3, 255))
#test(repr(cmyk(4%, 5%, 6%, 7%).values), repr((3.9%, 5.1%, 5.9%, 7.1%)))
#test(luma(40).values, (40,))

---
// Test stroke fields.
#test((1em + blue).paint, blue)
#test((1em + blue).thickness, 1em)
#test((1em + blue).line_cap, "butt")
#test((1em + blue).line_join, "miter")
#test((1em + blue).dash_pattern, none)
#test((1em + blue).miter_limit, 4.0)

---
// Test 2d alignment 'horizontal' field.
#test((start + top).horizontal, start)
#test((end + top).horizontal, end)
#test((left + top).horizontal, left)
#test((right + top).horizontal, right)
#test((center + top).horizontal, center)
#test((start + bottom).horizontal, start)
#test((end + bottom).horizontal, end)
#test((left + bottom).horizontal, left)
#test((right + bottom).horizontal, right)
#test((center + bottom).horizontal, center)
#test((start + horizon).horizontal, start)
#test((end + horizon).horizontal, end)
#test((left + horizon).horizontal, left)
#test((right + horizon).horizontal, right)
#test((center + horizon).horizontal, center)
#test((top + start).horizontal, start)
#test((bottom + end).horizontal, end)
#test((horizon + center).horizontal, center)

---
// Test 2d alignment 'vertical' field.
#test((start + top).vertical, top)
#test((end + top).vertical, top)
#test((left + top).vertical, top)
#test((right + top).vertical, top)
#test((center + top).vertical, top)
#test((start + bottom).vertical, bottom)
#test((end + bottom).vertical, bottom)
#test((left + bottom).vertical, bottom)
#test((right + bottom).vertical, bottom)
#test((center + bottom).vertical, bottom)
#test((start + horizon).vertical, horizon)
#test((end + horizon).vertical, horizon)
#test((left + horizon).vertical, horizon)
#test((right + horizon).vertical, horizon)
#test((center + horizon).vertical, horizon)
#test((top + start).vertical, top)
#test((bottom + end).vertical, bottom)
#test((horizon + center).vertical, horizon)

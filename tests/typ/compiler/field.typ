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
// Error: 6-13 dictionary does not contain key "invalid"
#(:).invalid

---
// Error: 8-10 cannot access fields on type boolean
#false.ok

---
// Error: 25-28 content does not contain field "fun"
#show heading: it => it.fun
= A

---
// Error: 9-13 cannot access fields on type boolean
#{false.true}

---
// Test relative length fields.
#test((100% + 2em + 2pt).ratio, 100%)
#test((100% + 2em + 2pt).length, 2em + 2pt)
#test((100% + 2pt).length, 2pt)
#test((100% + 2pt - 2pt).length, 0pt)
#test((56% + 2pt - 56%).ratio, 0%)

---
// Test length fields.
#test((1pt).em, 0.0)
#test((1pt).abs, 1pt)
#test((3em).em, 3.0)
#test((3em).abs, 0pt)
#test((2em + 2pt).em, 2.0)
#test((2em + 2pt).abs, 2pt)

---
// Test stroke fields for simple strokes.
#test((1em + blue).paint, blue)
#test((1em + blue).thickness, 1em)
#test((1em + blue).cap, auto)
#test((1em + blue).join, auto)
#test((1em + blue).dash, auto)
#test((1em + blue).miter-limit, auto)

---
// Test complex stroke fields.
#let r1 = rect(stroke: (paint: cmyk(1%, 2%, 3%, 4%), thickness: 4em + 2pt, cap: "round", join: "bevel", miter-limit: 5.0, dash: none))
#let r2 = rect(stroke: (paint: cmyk(1%, 2%, 3%, 4%), thickness: 4em + 2pt, cap: "round", join: "bevel", miter-limit: 5.0, dash: (3pt, "dot", 4em)))
#let r3 = rect(stroke: (paint: cmyk(1%, 2%, 3%, 4%), thickness: 4em + 2pt, cap: "round", join: "bevel", dash: (array: (3pt, "dot", 4em), phase: 5em)))
#let s1 = r1.stroke
#let s2 = r2.stroke
#let s3 = r3.stroke
#test(s1.paint, cmyk(1%, 2%, 3%, 4%))
#test(s1.thickness, 4em + 2pt)
#test(s1.cap, "round")
#test(s1.join, "bevel")
#test(s1.miter-limit, 5.0)
#test(s3.miter-limit, auto)
#test(s1.dash, none)
#test(s2.dash, (array: (3pt, "dot", 4em), phase: 0pt))
#test(s3.dash, (array: (3pt, "dot", 4em), phase: 5em))

---
// Test 2d alignment 'horizontal' field.
#test((start + top).x, start)
#test((end + top).x, end)
#test((left + top).x, left)
#test((right + top).x, right)
#test((center + top).x, center)
#test((start + bottom).x, start)
#test((end + bottom).x, end)
#test((left + bottom).x, left)
#test((right + bottom).x, right)
#test((center + bottom).x, center)
#test((start + horizon).x, start)
#test((end + horizon).x, end)
#test((left + horizon).x, left)
#test((right + horizon).x, right)
#test((center + horizon).x, center)
#test((top + start).x, start)
#test((bottom + end).x, end)
#test((horizon + center).x, center)

---
// Test 2d alignment 'vertical' field.
#test((start + top).y, top)
#test((end + top).y, top)
#test((left + top).y, top)
#test((right + top).y, top)
#test((center + top).y, top)
#test((start + bottom).y, bottom)
#test((end + bottom).y, bottom)
#test((left + bottom).y, bottom)
#test((right + bottom).y, bottom)
#test((center + bottom).y, bottom)
#test((start + horizon).y, horizon)
#test((end + horizon).y, horizon)
#test((left + horizon).y, horizon)
#test((right + horizon).y, horizon)
#test((center + horizon).y, horizon)
#test((top + start).y, top)
#test((bottom + end).y, bottom)
#test((horizon + center).y, horizon)

---
#{
  let object = sym.eq.not
  // Error: 3-9 cannot mutate fields on symbol
  object.property = "value"
}

---
#{
  let object = [hi]
  // Error: 3-9 cannot mutate fields on content
  object.property = "value"
}

---
#{
  let object = calc
  // Error: 3-9 cannot mutate fields on module
  object.property = "value"
}

---
#{
  let object = calc.sin
  // Error: 3-9 cannot mutate fields on function
  object.property = "value"
}

---
#{
  let object = none
  // Error: 3-9 none does not have accessible fields
  object.property = "value"
}

---
#{
  let object = 10
  // Error: 3-9 integer does not have accessible fields
  object.property = "value"
}

---
#{
  let s = 1pt + red
  // Error: 3-4 fields on stroke are not yet mutable
  // Hint: 3-4 try creating a new stroke with the updated field value instead
  s.thickness = 5pt
}

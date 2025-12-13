// Test dictionaries.

--- dict-basic-syntax paged ---

// Empty
#(:)

// Two pairs and string key.
#let dict = (normal: 1, "spacy key": 2)
#dict

#test(dict.normal, 1)
#test(dict.at("spacy key"), 2)

--- dict-fields paged ---
// Test field on dictionary.
#let dict = (nothing: "ness", hello: "world")
#test(dict.nothing, "ness")
#{
  let world = dict
    .hello

  test(world, "world")
}

--- dict-missing-field paged ---
// Error: 6-13 dictionary does not contain key "invalid"
#(:).invalid

--- dict-bad-key paged ---
// Error: 3-7 expected string, found boolean
// Error: 16-18 expected string, found integer
#(true: false, 42: 3)

--- dict-duplicate-key paged ---
// Error: 24-29 duplicate key: first
#(first: 1, second: 2, first: 3)

--- dict-duplicate-key-stringy paged ---
// Error: 17-20 duplicate key: a
#(a: 1, "b": 2, "a": 3)

--- dict-bad-expression paged ---
// Simple expression after already being identified as a dictionary.
// Error: 9-10 expected named or keyed pair, found identifier
#(a: 1, b)

--- dict-leading-colon paged ---
// Identified as dictionary due to initial colon.
// The boolean key is allowed for now since it will only cause an error at the evaluation stage.
// Error: 4-5 expected named or keyed pair, found integer
// Error: 17 expected expression
#(:1 b:"", true:)

--- spread-into-dict paged ---
#{
  let x = (a: 1)
  let y = (b: 2)
  let z = (a: 3)
  test((:..x, ..y, ..z), (a: 3, b: 2))
  test((..(a: 1), b: 2), (a: 1, b: 2))
}

--- spread-array-into-dict paged ---
// Error: 3-11 cannot spread array into dictionary
#(..(1, 2), a: 1)

--- dict-at-lvalue paged ---
// Test lvalue and rvalue access.
#{
  let dict = (a: 1, "b b": 1)
  dict.at("b b") += 1
  dict.state = (ok: true, err: false)
  test(dict, (a: 1, "b b": 2, state: (ok: true, err: false)))
  test(dict.state.ok, true)
  dict.at("state").ok = false
  test(dict.state.ok, false)
  test(dict.state.err, false)
}

--- dict-at-missing-key paged ---
// Test rvalue missing key.
#{
  let dict = (a: 1, b: 2)
  // Error: 11-23 dictionary does not contain key "c" and no default value was specified
  let x = dict.at("c")
}

--- dict-at-default paged ---
// Test default value.
#test((a: 1, b: 2).at("b", default: 3), 2)
#test((a: 1, b: 2).at("c", default: 3), 3)

--- dict-insert paged ---
// Test insert.
#{
  let dict = (a: 1, b: 2)
  dict.insert("b", 3)
  test(dict, (a: 1, b: 3))
  dict.insert("c", 5)
  test(dict, (a: 1, b: 3, c: 5))
}

--- dict-remove-with-default paged ---
// Test remove with default value.
#{
  let dict = (a: 1, b: 2)
  test(dict.remove("b", default: 3), 2)
}

#{
  let dict = (a: 1, b: 2)
  test(dict.remove("c", default: 3), 3)
}

--- dict-missing-lvalue paged ---
// Missing lvalue is not automatically none-initialized.
#{
  let dict = (:)
  // Error: 3-9 dictionary does not contain key "b"
  // Hint: 3-9 use `insert` to add or update values
  dict.b += 1
}

--- dict-basic-methods paged ---
// Test dictionary methods.
#let dict = (a: 3, c: 2, b: 1)
#test("c" in dict, true)
#test(dict.len(), 3)
#test(dict.values(), (3, 2, 1))
#test(dict.pairs().map(p => p.first() + str(p.last())).join(), "a3c2b1")

#test(dict.remove("c"), 2)
#test("c" in dict, false)
#test(dict, (a: 3, b: 1))

--- dict-from-module paged ---
// Test dictionary constructor
#test(type(dictionary(sys).at("version")), version)
#test(dictionary(sys).at("no-crash", default: none), none)

--- dict-remove-order paged ---
// Test that removal keeps order.
#let dict = (a: 1, b: 2, c: 3, d: 4)
#test(dict.remove("b"), 2)
#test(dict.keys(), ("a", "c", "d"))

--- dict-insert-order paged ---
#let dict = (a: 1, b: 2)
#let rhs = (c: 3, a: 4)

// Add
#test((dict + rhs).keys(), ("a", "b", "c"))

// Join
#test({ dict; rhs }.keys(), ("a", "b", "c"))

// Spread
#test((:..dict, ..rhs).keys(), ("a", "b", "c"))

// Insert
#{
  for (k, v) in rhs {
    dict.insert(k, v)
  }
  test(dict.keys(), ("a", "b", "c"))
}

// Assign
#{
  dict.a = 5
  dict.d = 6
  test(dict.keys(), ("a", "b", "c", "d"))
}

--- dict-temporary-lvalue paged ---
// Error: 3-15 cannot mutate a temporary value
#((key: "val").other = "some")

--- dict-function-item-not-a-method paged ---
#{
  let dict = (
    call-me: () => 1,
  )
  // Error: 8-15 type dictionary has no method `call-me`
  // Hint: 8-15 to call the function stored in the dictionary, surround the field access with parentheses, e.g. `(dict.call-me)(..)`
  dict.call-me()
}

--- dict-item-missing-method paged ---
#{
  let dict = (
    nonfunc: 1
  )

  // Error: 8-15 type dictionary has no method `nonfunc`
  // Hint: 8-15 did you mean to access the field `nonfunc`?
  dict.nonfunc()
}

--- dict-dynamic-duplicate-key paged ---
#let a = "hello"
#let b = "world"
#let c = "value"
#let d = "conflict"

#test(((a): b), ("hello": "world"))
#test(((a): 1, (a): 2), ("hello": 2))
#test((hello: 1, (a): 2), ("hello": 2))
#test((a + b: c, (a + b): d, (a): "value2", a: "value3"), ("helloworld": "conflict", "hello": "value2", "a": "value3"))

--- dict-filter paged ---
// Test the `filter` method.
#test((:).filter(calc.even), (:))
#test((a: 0, b: 1, c: 2).filter(v => v != 0), (b: 1, c: 2))
#test((a: 0, b: 1, c: 2).filter(calc.even), (a: 0, c: 2))

--- dict-filter-error paged ---
// Test that errors in the predicate are reported properly.
// Error: 23-28 cannot subtract integer from string
#(a: "a").filter(v => v - 2)

--- dict-map paged ---
// Test the `map` method.
#test(().map(x => x * 2), ())
#test((a: 2, b: 3).map(x => x * 2), (a: 4, b: 6))

--- dict-map-error paged ---
// Test that errors in the function are reported properly.
// Error: 20-25 cannot subtract integer from string
#(a: "a").map(v => v - 2)

--- issue-1338-dictionary-underscore paged ---
#let foo = "foo"
#let bar = "bar"
// Error: 8-9 expected expression, found underscore
// Error: 16-17 expected expression, found underscore
#(foo: _, bar: _)

--- issue-1342-dictionary-bare-expressions paged ---
// Error: 5-8 expected named or keyed pair, found identifier
// Error: 10-13 expected named or keyed pair, found identifier
#(: foo, bar)

--- issue-3154-dict-at-not-contained paged ---
#{
  let dict = (a: 1)
  // Error: 3-15 dictionary does not contain key "b" and no default value was specified
  dict.at("b")
}

--- issue-3154-dict-at-missing-default paged ---
#{
  let dict = (a: 1)
  test(dict.at("b", default: 0), 0)
}

--- issue-3154-dict-at-missing-mutable paged ---
#{
  let dict = (a: 1)
  // Error: 3-15 dictionary does not contain key "b"
  // Hint: 3-15 use `insert` to add or update values
  dict.at("b") = 9
}

--- issue-3154-dict-at-missing-mutable-default paged ---
#{
  let dict = (a: 1)
  // Error: 3-27 dictionary does not contain key "b"
  // Hint: 3-27 use `insert` to add or update values
  dict.at("b", default: 0) = 9
}

--- issue-3154-dict-syntax-missing paged ---
#{
  let dict = (a: 1)
  // Error: 8-9 dictionary does not contain key "b"
  dict.b
}

--- issue-3154-dict-syntax-missing-mutable paged ---
#{
  let dict = (a: 1)
  dict.b = 9
  test(dict, (a: 1, b: 9))
}

--- issue-3154-dict-syntax-missing-add-assign paged ---
#{
  let dict = (a: 1)
  // Error: 3-9 dictionary does not contain key "b"
  // Hint: 3-9 use `insert` to add or update values
  dict.b += 9
}

--- issue-3232-dict-unexpected-keys-sides paged ---
// Confusing "expected relative length or dictionary, found dictionary"
// Error: 16-58 unexpected keys "unexpected" and "unexpected-too"
#block(outset: (unexpected: 0.5em, unexpected-too: 0.2em), [Hi])

--- issue-3232-dict-unexpected-keys-corners paged ---
// Error: 14-56 unexpected keys "unexpected" and "unexpected-too"
#box(radius: (unexpected: 0.5em, unexpected-too: 0.5em), [Hi])

--- issue-3232-dict-unexpected-key-sides paged ---
// Error: 16-49 unexpected key "unexpected", valid keys are "left", "top", "right", "bottom", "x", "y", and "rest"
#block(outset: (unexpected: 0.2em, right: 0.5em), [Hi]) // The 1st key is unexpected

--- issue-3232-dict-unexpected-key-corners paged ---
// Error: 14-50 unexpected key "unexpected", valid keys are "top-left", "top-right", "bottom-right", "bottom-left", "left", "top", "right", "bottom", and "rest"
#box(radius: (top-left: 0.5em, unexpected: 0.5em), [Hi]) // The 2nd key is unexpected

--- issue-3232-dict-empty paged ---
#block(outset: (:), [Hi]) // Ok
#box(radius: (:), [Hi]) // Ok

// Test dictionaries.
// Ref: false

---
// Ref: true

// Empty
#(:)

// Two pairs and string key.
#let dict = (normal: 1, "spacy key": 2)
#dict

#test(dict.normal, 1)
#test(dict.at("spacy key"), 2)

---
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

---
// Test rvalue missing key.
#{
  let dict = (a: 1, b: 2)
  // Error: 11-23 dictionary does not contain key "c" and no default value was specified
  let x = dict.at("c")
}

---
// Test default value.
#test((a: 1, b: 2).at("b", default: 3), 2)
#test((a: 1, b: 2).at("c", default: 3), 3)

---
// Test remove with default value.
#{
  let dict = (a: 1, b: 2)
  test(dict.remove("b", default: 3), 2)
}

#{
  let dict = (a: 1, b: 2)
  test(dict.remove("c", default: 3), 3)
}

---
// Missing lvalue is not automatically none-initialized.
#{
  let dict = (:)
  // Error: 3-9 dictionary does not contain key "b" and no default value was specified
  dict.b += 1
}

---
// Test dictionary methods.
#let dict = (a: 3, c: 2, b: 1)
#test("c" in dict, true)
#test(dict.len(), 3)
#test(dict.values(), (3, 2, 1))
#test(dict.pairs().map(p => p.first() + str(p.last())).join(), "a3c2b1")

#dict.remove("c")
#test("c" in dict, false)
#test(dict, (a: 3, b: 1))

---
// Test that removal keeps order.
#let dict = (a: 1, b: 2, c: 3, d: 4)
#dict.remove("b")
#test(dict.keys(), ("a", "c", "d"))

---
// Error: 24-29 duplicate key: first
#(first: 1, second: 2, first: 3)

---
// Error: 17-20 duplicate key: a
#(a: 1, "b": 2, "a": 3)

---
// Simple expression after already being identified as a dictionary.
// Error: 9-10 expected named or keyed pair, found identifier
#(a: 1, b)

// Identified as dictionary due to initial colon.
// Error: 4-5 expected named or keyed pair, found integer
// Error: 5 expected comma
// Error: 12-16 expected identifier or string, found boolean
// Error: 17 expected expression
#(:1 b:"", true:)

// Error: 3-8 expected identifier or string, found binary expression
#(a + b: "hey")

---
// Error: 3-15 cannot mutate a temporary value
#((key: "val").other = "some")

---
#{
  let dict = (
    func: () => 1,
  )
  // Error: 8-12 type dictionary has no method `func`
  // Hint: 8-12 to call the function stored in the dictionary, surround the field access with parentheses
  dict.func()
}

---
#{
  let dict = (
    nonfunc: 1
  )

  // Error: 8-15 type dictionary has no method `nonfunc`
  dict.nonfunc()
}

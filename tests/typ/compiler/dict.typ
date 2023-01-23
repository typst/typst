// Test dictionaries.
// Ref: false

---
// Ref: true

// Empty
{(:)}

// Two pairs and string key.
#let dict = (normal: 1, "spacy key": 2)
#dict

#test(dict.normal, 1)
#test(dict.at("spacy key"), 2)

---
// Test lvalue and rvalue access.
{
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
{
  let dict = (a: 1, b: 2)
  // Error: 11-23 dictionary does not contain key "c"
  let x = dict.at("c")
}

---
// Missing lvalue is not automatically none-initialized.
{
  let dict = (:)
  // Error: 3-9 dictionary does not contain key "b"
  dict.b += 1
}

---
// Test dictionary methods.
#let dict = (a: 3, c: 2, b: 1)
#test("c" in dict, true)
#test(dict.len(), 3)
#test(dict.values(), (3, 1, 2))
#test(dict.pairs((k, v) => k + str(v)).join(), "a3b1c2")

{ dict.remove("c") }
#test("c" in dict, false)
#test(dict, (a: 3, b: 1))

---
// Error: 24-29 duplicate key
{(first: 1, second: 2, first: 3)}

---
// Error: 17-20 duplicate key
{(a: 1, "b": 2, "a": 3)}

---
// Simple expression after already being identified as a dictionary.
// Error: 9-10 expected named or keyed pair, found identifier
{(a: 1, b)}

// Identified as dictionary due to initial colon.
// Error: 4-5 expected named or keyed pair, found integer
// Error: 5 expected comma
// Error: 12-16 expected identifier or string, found boolean
// Error: 17 expected expression
{(:1 b:"", true:)}

// Error: 3-8 expected identifier or string, found binary expression
{(a + b: "hey")}

---
// Error: 3-15 cannot mutate a temporary value
{ (key: "val").other = "some" }

---
{
  let object = none
  // Error: 3-9 expected dictionary, found none
  object.property = "value"
}

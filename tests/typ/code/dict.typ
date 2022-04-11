// Test dictionaries.
// Ref: false

---
// Ref: true

// Empty
{(:)}

// Two pairs.
{(a1: 1, a2: 2)}

---
// Test lvalue and rvalue access.
{
  let dict = (a: 1, b: 1)
  dict("b") += 1
  dict("c") = 3
  test(dict, (a: 1, b: 2, c: 3))
}

---
// Test rvalue missing key.
{
  let dict = (a: 1, b: 2)
  // Error: 11-20 dictionary does not contain key: "c"
  let x = dict("c")
}

---
// Missing lvalue is automatically none-initialized.
{
  let dict = (:)
  // Error: 3-17 cannot add none and integer
  dict("b") += 1
}

---
// Error: 24-32 pair has duplicate key
{(first: 1, second: 2, first: 3)}

---
// Simple expression after already being identified as a dictionary.
// Error: 9-10 expected named pair, found expression
{(a: 1, b)}

// Identified as dictionary due to initial colon.
// Error: 4-5 expected named pair, found expression
// Error: 5 expected comma
// Error: 12-16 expected identifier
// Error: 17-18 expected expression, found colon
{(:1 b:"", true::)}

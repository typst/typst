// Test while expressions.

---
// Should output `2 4 6 8 10`.
#let i = 0
#while i < 10 [
  { i += 2 }
  #i
]

// Should output `Hi`.
#let iter = true
#while iter {
  iter = false
  "Hi."
}

#while false {
  dont-care
}

---
// Value of while loops.
// Ref: false

#test(while false {}, none)

#let i = 0
#test(type(while i < 1 [{ i += 1 }]), "content")

---
// Condition must be boolean.
// Error: 8-14 expected boolean, found content
#while [nope] [nope]

---
// Make sure that we terminate and don't complain multiple times.
#while true {
  // Error: 3-7 unknown variable
  nope
}

---
// Error: 7 expected expression
#while

// Error: 7 expected expression
{while}

// Error: 9 expected body
#while x

// Error: 7 expected expression
#while
x {}

// Error: 9 expected body
#while x something

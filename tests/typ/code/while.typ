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
#test(type(while i < 1 [{ i += 1 }]), "template")

---
// Ref: false

// Condition must be boolean.
// Error: 8-14 expected boolean, found template
#while [nope] [nope]

// Make sure that we don't complain twice.
// Error: 8-15 unknown variable
#while nothing {}

// Errors taint everything.
#let i = 0
#test(error, while i < 10 {
    i += 1
    if i < 5 [nope] else { error }
})
#test(i, 10)

---
// Error: 7 expected expression
#while

// Error: 7 expected expression
{while}

// Error: 9 expected body
#while x

// Should output `x`.
// Error: 7 expected expression
#while
x {}

// Should output `something`.
// Error: 9 expected body
#while x something

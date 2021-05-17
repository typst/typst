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
#test(type(while false {}), "template")
#test(type(while false []), "template")

---
// Condition must be boolean.
// Error: 8-14 expected boolean, found template
#while [nope] [nope]

// Make sure that we don't complain twice.
// Error: 8-15 unknown variable
#while nothing {}

// A single error stops iteration.
#let i = 0
#test(error, while i < 10 {
    i += 1
    if i < 5 [nope] else { error }
})
#test(i, 5)

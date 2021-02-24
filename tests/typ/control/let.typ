// Test let bindings.

---
// Ref: false

// Automatically initialized with none.
#let x
#test(x, none)

// Manually initialized with one.
#let x = 1
#test(x, 1)

---
// Termination.

// Terminated by line break.
#let v1 = 1
One

// Terminated by semicolon.
#let v2 = 2; Two

// Terminated by semicolon and line break.
#let v3 = 3;
Three

// Terminated because expression ends.
// Error: 12 expected semicolon or line break
#let v4 = 4 Four

// Terminated by semicolon even though we are in a paren group.
// Error: 2:19 expected expression
// Error: 1:19 expected closing paren
#let v5 = (1, 2 + ; Five

#test(v1, 1)
#test(v2, 2)
#test(v3, 3)
#test(v4, 4)
#test(v5, (1, 2))

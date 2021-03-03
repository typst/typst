// Test let bindings.

---
// Ref: false

// Automatically initialized with none.
#let x
#test(x, none)

// Error: 9 expected expression
#let y =
#test(y, none)

// Manually initialized with one.
#let z = 1
#test(z, 1)

---
// Syntax sugar for function definitions.
#let background = #239dad
#let box(body) = box(width: 2cm, height: 1cm, color: background, body)
#box[Hi!]

// Error: 13 expected body
#let func(x)

// Error: 2-6 unknown variable
{func}

// Error: 15 expected expression
#let func(x) =

// Error: 2-6 unknown variable
{func}

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

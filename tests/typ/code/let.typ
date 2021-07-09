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
#let fill = conifer
#let rect(body) = rect(width: 2cm, fill: fill, pad(5pt, body))
#rect[Hi!]

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
// Error: 19 expected expression
// Error: 19 expected closing paren
#let v5 = (1, 2 + ; Five

#test(v1, 1)
#test(v2, 2)
#test(v3, 3)
#test(v4, 4)
#test(v5, (1, 2))

---
// Error: 5 expected identifier
#let

// Error: 5 expected identifier
{let}

// Error: 6-9 expected identifier, found string
#let "v"

// Should output `1`.
// Error: 7 expected semicolon or line break
#let v 1

// Error: 9 expected expression
#let v =

// Should output a heading `1`.
// Error: 6-9 expected identifier, found string
#let "v" = 1

// Test let bindings.

---
// Automatically initialized with none.
#let x
#test(x, none)

// Manually initialized with one.
#let z = 1
#test(z, 1)

// Syntax sugar for function definitions.
#let fill = conifer
#let f(body) = rect(width: 2cm, fill: fill, inset: 5pt, body)
#f[Hi!]

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

#test(v1, 1)
#test(v2, 2)
#test(v3, 3)

---
// Destructuring.

// Simple destructuring.
#let (a, b) = (1, 2)
#test(a, 1)
#test(b, 2)

// Destructuring with multiple placeholders.
#let (a, _, c, _) = (1, 2, 3, 4)
#test(a, 1)
#test(c, 3)

// Destructuring with a sink.
#let (a, b, ..c) = (1, 2, 3, 4, 5, 6)
#test(a, 1)
#test(b, 2)
#test(c, (3, 4, 5, 6))

// Destructuring with a sink in the middle.
#let (a, ..b, c) = (1, 2, 3, 4, 5, 6)
#test(a, 1)
#test(b, (2, 3, 4, 5))
#test(c, 6)

// Destructuring with an empty sink.
#let (..a, b, c) = (1, 2)
#test(a, ())

#let (a, ..b, c) = (1, 2)
#test(b, ())

#let (a, b, ..c) = (1, 2)
#test(c, ())

// Destructuring with an empty sink and empty array.
#let (..a) = ()
#test(a, ())

// Destructuring with unnamed sink.
#let (a, .., b) = (1, 2, 3, 4)
#test(a, 1)
#test(b, 4)

// Error: 10-11 at most one binding per identifier is allowed
#let (a, a) = (1, 2)

// Error: 12-15 at most one destructuring sink is allowed
#let (..a, ..a) = (1, 2)

---
// Error: 13-14 not enough elements to destructure
#let (a, b, c) = (1, 2)

---
// Error: 6-9 too many elements to destructure
#let (a) = (1, 2)

---
// Error: 6-20 not enough elements to destructure
#let (..a, b, c, d) = (1, 2)

---
// Error: 6-12 cannot destructure boolean
#let (a, b) = true

---
// Error: 5 expected identifier
#let

// Error: 6 expected identifier
#{let}

// Error: 5 expected identifier
// Error: 5 expected semicolon or line break
#let "v"

// Error: 7 expected semicolon or line break
#let v 1

// Error: 9 expected expression
#let v =

// Error: 5 expected identifier
// Error: 5 expected semicolon or line break
#let "v" = 1

// Terminated because expression ends.
// Error: 12 expected semicolon or line break
#let v4 = 4 Four

// Terminated by semicolon even though we are in a paren group.
// Error: 18 expected expression
// Error: 18 expected closing paren
#let v5 = (1, 2 + ; Five

---
// Error: 13 expected equals sign
#let func(x)

// Error: 15 expected expression
#let func(x) =

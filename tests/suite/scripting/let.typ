// Test let bindings.

--- let-basic paged ---
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

--- let-termination paged ---
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

--- let-valid-idents paged ---
// Test what constitutes a valid Typst identifier.
#let name = 1
#test(name, 1)
#let name_ = 1
#test(name_, 1)
#let name-2 = 1
#test(name-2, 1)
#let name_2 = 1
#test(name_2, 1)
#let __name = 1
#test(__name, 1)
#let ůñıćóðė = 1
#test(ůñıćóðė, 1)

--- let-binding-keyword-in-markup paged ---
// Error: 6-8 expected pattern, found keyword `as`
// Hint: 6-8 keyword `as` is not allowed as an identifier; try `as_` instead
#let as = 1 + 2

--- let-binding-keyword-in-code paged ---
#{
  // Error: 7-9 expected pattern, found keyword `as`
  // Hint: 7-9 keyword `as` is not allowed as an identifier; try `as_` instead
  let as = 10
}

--- let-ident-parenthesized paged ---
// Test parenthesised assignments.
#let (a) = (1, 2)

--- let-incomplete paged ---
// Error: 5 expected pattern
#let

// Error: 6 expected pattern
#{let}

// Error: 6-9 expected pattern, found string
#let "v"

// Error: 7 expected semicolon or line break
#let v 1

// Error: 9 expected expression
#let v =

// Error: 6-9 expected pattern, found string
#let "v" = 1

// Terminated because expression ends.
// Error: 12 expected semicolon or line break
#let v4 = 4 Four

// Terminated by semicolon even though we are in a paren group.
// Error: 18 expected expression
// Error: 11-12 unclosed delimiter
#let v5 = (1, 2 + ; Five

// Error: 9-13 expected pattern, found boolean
#let (..true) = false

--- underscore-invalid paged ---
#let _ = 4

#for _ in range(2) []

// Error: 2-3 unexpected underscore
#_

// Error: 8-9 expected expression, found underscore
#lorem(_)

// Error: 3-4 expected expression, found underscore
#(_,)

// Error: 3-4 expected expression, found underscore
#{_}

// Error: 8-9 expected expression, found underscore
#{ 1 + _ }

--- let-function-incomplete paged ---
// Error: 13 expected equals sign
#let func(x)

// Error: 15 expected expression
#let func(x) =

--- let-function-parenthesized paged ---
// This is not yet parsed in the ideal way.
// Error: 12 expected equals sign
#let (func)(x)

--- let-function-parenthesized-with-init paged ---
// These errors aren't great.
// Error: 12 expected equals sign
// Error: 15-15 expected semicolon or line break
#let (func)(x) = 3

--- let-with-no-init-group paged ---
// This was unintentionally allowed ...
// Error: 9 expected equals sign
#let (a)

--- let-with-no-init-destructuring paged ---
// ... where this wasn't.
// Error: 12 expected equals sign
#let (a, b)

--- issue-4027-let-binding-with-keyword-context paged ---
// Error: 6-13 expected pattern, found keyword `context`
// Hint: 6-13 keyword `context` is not allowed as an identifier; try `context_` instead
#let context = 5

--- issue-4027-let-binding-with-keyword-let paged ---
// Error: 6-9 expected pattern, found keyword `let`
// Hint: 6-9 keyword `let` is not allowed as an identifier; try `let_` instead
#let let = 5

--- issue-4027-let-binding-with-destructured-keywords paged ---
// Error: 7-14 expected pattern, found keyword `context`
// Hint: 7-14 keyword `context` is not allowed as an identifier; try `context_` instead
// Error: 21-24 expected pattern, found keyword `let`
// Hint: 21-24 keyword `let` is not allowed as an identifier; try `let_` instead
#let (context, foo, let) = (5, 6, 7)

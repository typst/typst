// Test if-else expressions.

--- if-markup paged ---
// Test condition evaluation.
#if 1 < 2 [
  One.
]

#if true == false [
  {Bad}, but we {dont-care}!
]

--- if-condition-complex paged ---
// Braced condition.
#if {true} [
  One.
]

// Content block in condition.
#if [] != none [
  Two.
]

// Multi-line condition with parens.
#if (
  1 + 1
    == 1
) [
  Nope.
] else {
  "Three."
}

// Multiline.
#if false [
  Bad.
] else {
  let point = "."
  "Four" + point
}

// Content block can be argument or body depending on whitespace.
#{
  if content == type[b] [Fi] else [Nope]
  if content == type [Nope] else [ve.]
}

#let i = 3
#if i < 2 [
  Five.
] else if i < 4 [
  Six.
] else [
  Seven.
]

--- if-else-if-else paged ---
// Test else if.

#let nth(n) = {
  str(n)
  if n == 1 { "st" }
  else if n == 2 { "nd" }
  else if n == 3 { "rd" }
  else { "th" }
}

#test(nth(1), "1st")
#test(nth(2), "2nd")
#test(nth(3), "3rd")
#test(nth(4), "4th")
#test(nth(5), "5th")

--- if-expression paged ---
// Value of if expressions.

#{
  let x = 1
  let y = 2
  let z

  // Returns if branch.
  z = if x < y { "ok" }
  test(z, "ok")

  // Returns else branch.
  z = if x > y { "bad" } else { "ok" }
  test(z, "ok")

  // Missing else evaluates to none.
  z = if x > y { "bad" }
  test(z, none)
}

--- if-condition-string-invalid paged ---
// Condition must be boolean.
// If it isn't, neither branch is evaluated.
// Error: 5-14 expected boolean, found string
#if "a" + "b" { nope } else { nope }

--- if-condition-invalid-and-wrong-type paged ---
// Make sure that we don't complain twice.
// Error: 5-12 cannot add integer and string
#if 1 + "2" {}

--- if-incomplete paged ---
// Error: 4 expected expression
#if

// Error: 5 expected expression
#{if}

// Error: 6 expected block
#if x

// Error: 2-6 unexpected keyword `else`
#else {}

// Should output `x`.
// Error: 4 expected expression
#if
x {}

// Should output `something`.
// Error: 6 expected block
#if x something

// Should output `A thing.`
// Error: 19 expected block
A#if false {} else thing

#if a []else [b]
#if a [] else [b]
#if a {} else [b]

// Test while expressions.

--- while-loop-basic paged ---
// Should output `2 4 6 8 10`.
#let i = 0
#while i < 10 [
  #(i += 2)
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

--- while-loop-expr paged ---
// Value of while loops.

#test(while false {}, none)

#let i = 0
#test(type(while i < 1 [#(i += 1)]), content)

--- while-loop-condition-content-invalid paged ---
// Condition must be boolean.
// Error: 8-14 expected boolean, found content
#while [nope] [nope]

--- while-loop-condition-always-true paged ---
// Error: 8-25 condition is always true
#while 2 < "hello".len() {}

--- while-loop-limit paged ---
// Error: 2:2-2:24 loop seems to be infinite
#let i = 1
#while i > 0 { i += 1 }

--- while-loop-incomplete paged ---
// Error: 7 expected expression
#while

// Error: 8 expected expression
#{while}

// Error: 9 expected block
#while x

// Error: 7 expected expression
#while
x {}

// Error: 9 expected block
#while x something

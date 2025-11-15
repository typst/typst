// Test vectors.

--- math-vec-gap ---
#set math.vec(gap: 1em)
$ vec(1, 2) $

--- math-vec-align ---
$ vec(-1, 1, -1, align: #left)
  vec(-1, 1, -1, align: #center)
  vec(-1, 1, -1, align: #right) $

--- math-vec-align-explicit-alternating ---
// Test alternating alignment in a vector.
$ vec(
  "a" & "a a a" & "a a",
  "a a" & "a a" & "a",
  "a a a" & "a" & "a a a",
) $

--- math-vec-wide ---
// Test wide cell.
$ v = vec(1, 2+3, 4) $

--- math-vec-delim-set ---
// Test alternative delimiter.
#set math.vec(delim: "[")
$ vec(1, 2) $

--- math-vec-delim-empty-string ---
// Error: 22-24 expected exactly one character
#set math.vec(delim: "")

--- math-vec-delim-not-single-char ---
// Error: 22-39 expected exactly one character
#set math.vec(delim: "not a delimiter")

--- math-vec-delim-invalid-char ---
// Error: 22-25 invalid delimiter: "%"
#set math.vec(delim: "%")

--- math-vec-delim-invalid-symbol ---
// Error: 22-33 invalid delimiter: "%"
#set math.vec(delim: sym.percent)

--- math-vec-delim-invalid-opening ---
// Error: 22-33 invalid delimiter: "%"
#set math.vec(delim: ("%", none))

--- math-vec-delim-invalid-closing ---
// Error: 22-33 invalid delimiter: "%"
#set math.vec(delim: (none, "%"))

--- math-vec-linebreaks ---
// Warning: 20-29 linebreaks are ignored in elements
// Hint: 20-29 use commas instead to separate each line
$ vec(a, b, c) vec(a \ b \ c) $

--- math-vec-delim-class ---
// Test that delimiters have opening and closing math class.
$ 2vec(a, delim: bar.v) 2 $
$ 2 vec(a, delim: bar.v)2 $

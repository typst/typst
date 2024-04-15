// Test vectors.

--- math-vec-gap ---
#set math.vec(gap: 1em)
$ vec(1, 2) $


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

--- math-vec-delim-invalid ---
// Error: 22-25 expected "(", "[", "{", "|", "||", or none
#set math.vec(delim: "%")

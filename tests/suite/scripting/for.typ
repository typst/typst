// Test for loops.

--- for-loop-basic render ---

// Empty array.
#for x in () [Nope]

// Dictionary is traversed in insertion order.
// Should output `Name: Typst. Age: 2.`.
#for (k, v) in (Name: "Typst", Age: 2) [
  #k: #v.
]

// Block body.
// Should output `[1st, 2nd, 3rd, 4th]`.
#{
  "["
  for v in (1, 2, 3, 4) {
    if v > 1 [, ]
    [#v]
    if v == 1 [st]
    if v == 2 [nd]
    if v == 3 [rd]
    if v >= 4 [th]
   }
   "]"
}

// Content block body.
// Should output `2345`.
#for v in (1, 2, 3, 4, 5, 6, 7) [#if v >= 2 and v <= 5 { repr(v) }]

// Map captured arguments.
#let f1(..args) = args.pos().map(repr)
#let f2(..args) = args.named().pairs().map(p => repr(p.first()) + ": " + repr(p.last()))
#let f(..args) = (f1(..args) + f2(..args)).join(", ")
#f(1, a: 2)

--- for-loop-integrated render ---
#let out = ()

// Values of array.
#for v in (1, 2, 3) {
  out += (v,)
}

// Indices and values of array.
#for (i, v) in ("1", "2", "3").enumerate() {
  test(repr(i + 1), v)
}

// Pairs of dictionary.
#for v in (a: 4, b: 5) {
  out += (v,)
}

// Keys and values of dictionary.
#for (k, v) in (a: 6, b: 7) {
  out += (k,)
  out += (v,)
}

#test(out, (1, 2, 3, ("a", 4), ("b", 5), "a", 6, "b", 7))

// Grapheme clusters of string.
#let first = true
#let joined = for c in "abc👩‍👩‍👦‍👦" {
  if not first { ", " }
  first = false
  c
}

#test(joined, "a, b, c, 👩‍👩‍👦‍👦")

// Return value.
#test(for v in "" [], none)
#test(type(for v in "1" []), content)

--- for-loop-over-bool render ---
// Uniterable expression.
// Error: 11-15 cannot loop over boolean
#for v in true {}

--- for-loop-over-string render ---
// Keys and values of strings.
// Error: 6-12 cannot destructure values of string
#for (k, v) in "hi" {
  dont-care
}

--- for-loop-destructuring-without-parentheses render ---
// Destructuring without parentheses.
// Error: 7-8 unexpected comma
// Hint: 7-8 destructuring patterns must be wrapped in parentheses
#for k, v in (a: 4, b: 5) {
  dont-care
}

--- for-loop-destructuring-half render ---
// Error: 7-8 unexpected comma
// Hint: 7-8 destructuring patterns must be wrapped in parentheses
#for k, in () {}

--- for-loop-incomplete render ---
// Error: 5 expected pattern
#for

// Error: 5 expected pattern
#for//

// Error: 6 expected pattern
#{for}

// Error: 7 expected keyword `in`
#for v

// Error: 10 expected expression
#for v in

// Error: 15 expected block
#for v in iter

// Error: 5 expected pattern
#for
v in iter {}

// Error: 7-10 expected pattern, found string
// Error: 16 expected block
A#for "v" thing

// Error: 6-9 expected pattern, found string
#for "v" in iter {}

// Error: 7 expected keyword `in`
#for a + b in iter {}

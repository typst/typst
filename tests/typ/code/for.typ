// Test for loops.
// Ref: false

---
// Ref: true

// Empty array.
#for x in () [Nope]

// Dictionary is not traversed in insertion order.
// Should output `Age: 2. Name: Typst.`.
#for k, v in (Name: "Typst", Age: 2) [
  {k}: {v}.
]

// Block body.
// Should output `[1st, 2nd, 3rd, 4th]`.
{
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

// Loop over captured arguments.
#let f1(..args) = for v in args { (repr(v),) }
#let f2(..args) = for k, v in args { (repr(k) + ": " + repr(v),) }
#let f(..args) = join(sep: ", ", ..f1(..args), ..f2(..args))
#f(1, a: 2)

---
#let out = ()

// Values of array.
#for v in (1, 2, 3) {
  out += (v,)
}

// Indices and values of array.
#for i, v in ("1", "2", "3") {
  test(repr(i + 1), v)
}

// Values of dictionary.
#for v in (a: 4, b: 5) {
  out += (v,)
}

// Keys and values of dictionary.
#for k, v in (a: 6, b: 7) {
  out += (k,)
  out += (v,)
}

#test(out, (1, 2, 3, 4, 5, "a", 6, "b", 7))

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
#test(type(for v in "1" []), "content")

---
// Uniterable expression.
// Error: 11-15 cannot loop over boolean
#for v in true {}

---
// Keys and values of strings.
// Error: 6-10 mismatched pattern
#for k, v in "hi" {
  dont-care
}

---
// Error: 5 expected identifier
#for

// Error: 7 expected identifier
#for//

// Error: 5 expected identifier
{for}

// Error: 7 expected keyword `in`
#for v

// Error: 10 expected expression
#for v in

// Error: 15 expected body
#for v in iter

// Error: 5 expected identifier
#for
v in iter {}

// Error: 7-10 expected identifier, found string
A#for "v" thing

// Error: 6-9 expected identifier, found string
#for "v" in iter {}

// Error: 7 expected keyword `in`
#for a + b in iter {}

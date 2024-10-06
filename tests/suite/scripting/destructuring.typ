--- destructuring-group-1 ---
// This wasn't allowed.
#let ((x)) = 1
#test(x, 1)

--- destructuring-group-2 ---
// This also wasn't allowed.
#let ((a, b)) = (1, 2)
#test(a, 1)
#test(b, 2)

--- destructuring-dict-underscore ---
// Here, `best` was accessed as a variable, where it shouldn't have.
#{
  (best: _) = (best: "brr")
}

--- destructuring-dict-array-at ---
// Same here.
#{
  let array = (1, 2, 3, 4)
  (test: array.at(1), best: _) = (test: "baz", best: "brr")
  test(array, (1, "baz", 3, 4))
}

--- destructuring-dict-bad ---
// Error: 7-10 expected identifier, found group
// Error: 12-14 expected pattern, found integer
#let ((a): 10) = "world"

--- destructuring-bad-duplicate ---
// Here, `a` is not duplicate, where it was previously identified as one.
#let f((a: b), (c,), a) = (a, b, c)
#test(f((a: 1), (2,), 3), (3, 1, 2))

--- destructuring-non-atomic ---
// Ensure that we can't have non-atomic destructuring.
#let x = 1
#let c = [#() = ()]
#test(c.children.last(), [()])

--- destructuring-let-array ---
// Simple destructuring.
#let (a, b) = (1, 2)
#test(a, 1)
#test(b, 2)

--- destructuring-let-array-single-item ---
#let (a,) = (1,)
#test(a, 1)

--- destructuring-let-array-placeholders ---
// Destructuring with multiple placeholders.
#let (a, _, c, _) = (1, 2, 3, 4)
#test(a, 1)
#test(c, 3)

--- destructuring-let-array-with-sink-at-end ---
// Destructuring with a sink.
#let (a, b, ..c) = (1, 2, 3, 4, 5, 6)
#test(a, 1)
#test(b, 2)
#test(c, (3, 4, 5, 6))

--- destructuring-let-array-with-sink-in-middle ---
// Destructuring with a sink in the middle.
#let (a, ..b, c) = (1, 2, 3, 4, 5, 6)
#test(a, 1)
#test(b, (2, 3, 4, 5))
#test(c, 6)

--- destructuring-let-array-with-sink-at-start-empty ---
// Destructuring with an empty sink.
#let (..a, b, c) = (1, 2)
#test(a, ())
#test(b, 1)
#test(c, 2)

--- destructuring-let-array-with-sink-in-middle-empty ---
// Destructuring with an empty sink.
#let (a, ..b, c) = (1, 2)
#test(a, 1)
#test(b, ())
#test(c, 2)

--- destructuring-let-array-with-sink-at-end-empty ---
// Destructuring with an empty sink.
#let (a, b, ..c) = (1, 2)
#test(a, 1)
#test(b, 2)
#test(c, ())

--- destructuring-let-array-with-sink-empty ---
// Destructuring with an empty sink and empty array.
#let (..a) = ()
#test(a, ())

--- destructuring-let-array-with-unnamed-sink ---
// Destructuring with unnamed sink.
#let (a, .., b) = (1, 2, 3, 4)
#test(a, 1)
#test(b, 4)

// Error: 10-11 duplicate binding: a
#let (a, a) = (1, 2)

// Error: 12-15 only one destructuring sink is allowed
#let (..a, ..a) = (1, 2)

// Error: 12-13 duplicate binding: a
#let (a, ..a) = (1, 2)

// Error: 13-14 duplicate binding: a
#let (a: a, a) = (a: 1, b: 2)

// Error: 13-20 expected pattern, found function call
#let (a, b: b.at(0)) = (a: 1, b: 2)

// Error: 7-14 expected pattern, found function call
#let (a.at(0),) = (1,)

--- destructuring-let-empty-array ---
#let () = ()

--- destructuring-let-empty-array-too-many-elements ---
// Error: 6-8 too many elements to destructure
// Hint: 6-8 the provided array has a length of 2, but the pattern expects an empty array
#let () = (1, 2)

--- destructuring-let-array-too-few-elements ---
// Error: 13-14 not enough elements to destructure
// Hint: 13-14 the provided array has a length of 2, but the pattern expects 3 elements
#let (a, b, c) = (1, 2)

--- destructuring-let-array-too-few-elements-with-sink ---
// Error: 7-10 not enough elements to destructure
// Hint: 7-10 the provided array has a length of 2, but the pattern expects 4 elements
#let (..a, b, c, d) = (1, 2)

--- destructuring-let-array-bool-invalid ---
// Error: 6-12 cannot destructure boolean
#let (a, b) = true

--- destructuring-let-dict ---
// Simple destructuring.
#let (a: a, b, x: c) = (a: 1, b: 2, x: 3)
#test(a, 1)
#test(b, 2)
#test(c, 3)

--- destructuring-let-dict-with-sink-at-end ---
// Destructuring with a sink.
#let (a: _, ..b) = (a: 1, b: 2, c: 3)
#test(b, (b: 2, c: 3))

--- destructuring-let-dict-with-sink-in-middle ---
// Destructuring with a sink in the middle.
#let (a: _, ..b, c: _) = (a: 1, b: 2, c: 3)
#test(b, (b: 2))

--- destructuring-let-dict-with-sink-at-end-empty ---
// Destructuring with an empty sink.
#let (a: _, ..b) = (a: 1)
#test(b, (:))

--- destructuring-let-dict-with-sink-empty ---
// Destructuring with an empty sink and empty dict.
#let (..a) = (:)
#test(a, (:))

--- destructuring-let-dict-with-unnamed-sink ---
// Destructuring with unnamed sink.
#let (a, ..) = (a: 1, b: 2)
#test(a, 1)

--- destructuring-let-nested ---
// Nested destructuring.
#let ((a, b), (key: c)) = ((1, 2), (key: 3))
#test((a, b, c), (1, 2, 3))

--- destructuring-let-dict-key-string-invalid ---
// Keyed destructuring is not currently supported.
// Error: 7-18 expected pattern, found string
#let ("spacy key": val) = ("spacy key": 123)
#val

--- destructuring-let-dict-key-expr-invalid ---
// Keyed destructuring is not currently supported.
#let x = "spacy key"
// Error: 7-10 expected identifier, found group
#let ((x): v) = ("spacy key": 123)

--- destructuring-let-array-trailing-placeholders ---
// Trailing placeholders.
// Error: 10-11 not enough elements to destructure
// Hint: 10-11 the provided array has a length of 1, but the pattern expects 5 elements
#let (a, _, _, _, _) = (1,)
#test(a, 1)

--- destructuring-let-dict-patterns-invalid ---
// Error: 10-13 expected pattern, found string
// Error: 18-19 expected pattern, found integer
#let (a: "a", b: 2) = (a: 1, b: 2)

--- destructuring-let-dict-shorthand-missing-key ---
// Error: 10-11 dictionary does not contain key "b"
#let (a, b) = (a: 1)

--- destructuring-let-dict-missing-key ---
// Error: 10-11 dictionary does not contain key "b"
#let (a, b: b) = (a: 1)

--- destructuring-let-dict-from-array ---
// Error: 7-11 cannot destructure named pattern from an array
#let (a: a, b) = (1, 2, 3)

--- destructuring-during-loop-continue ---
// Test continue while destructuring.
// Should output "one = I \ two = II \ one = I".
#for num in (1, 2, 3, 1) {
  let (word, roman) = if num == 1 {
    ("one", "I")
  } else if num == 2 {
    ("two", "II")
  } else {
    continue
  }
  [#word = #roman \ ]
}

--- destructuring-assign ---
// Test destructuring assignments.

#let a = none
#let b = none
#let c = none
#((a,) = (1,))
#test(a, 1)

#((_, a, b, _) = (1, 2, 3, 4))
#test(a, 2)
#test(b, 3)

#((a, b, ..c) = (1, 2, 3, 4, 5, 6))
#test(a, 1)
#test(b, 2)
#test(c, (3, 4, 5, 6))

#((a: a, b, x: c) = (a: 1, b: 2, x: 3))
#test(a, 1)
#test(b, 2)
#test(c, 3)

#let a = (1, 2)
#((a: a.at(0), b) = (a: 3, b: 4))
#test(a, (3, 2))
#test(b, 4)

#let a = (1, 2)
#((a.at(0), b) = (3, 4))
#test(a, (3, 2))
#test(b, 4)

#((a, ..b) = (1, 2, 3, 4))
#test(a, 1)
#test(b, (2, 3, 4))

#let a = (1, 2)
#((b, ..a.at(0)) = (1, 2, 3, 4))
#test(a, ((2, 3, 4), 2))
#test(b, 1)

--- destructuring-assign-commas ---
// Test comma placement in destructuring assignment.
#let array = (1, 2, 3)
#((key: array.at(1)) = (key: "hi"))
#test(array, (1, "hi", 3))

#let array = (1, 2, 3)
#((array.at(1)) = ("hi"))
#test(array, (1, "hi", 3))

#let array = (1, 2, 3)
#((array.at(1),) = ("hi",))
#test(array, (1, "hi", 3))

#let array = (1, 2, 3)
#((array.at(1)) = ("hi",))
#test(array, (1, ("hi",), 3))

--- destructuring-assign-nested ---
// Test nested destructuring assignment.
#let a
#let b
#let c
#(((a, b), (key: c)) = ((1, 2), (key: 3)))
#test((a, b, c), (1, 2, 3))

--- destructuring-assign-nested-invalid ---
#let array = (1, 2, 3)
// Error: 3-17 cannot destructure string
#((array.at(1),) = ("hi"))
#test(array, (1, ("hi",), 3))

--- issue-3275-normal-variable ---
// Normal variable.
#for x in (1, 2) {}
#for x in (a: 1, b: 2) {}
#for x in "foo" {}
#for x in bytes("😊") {}

--- issue-3275-placeholder ---
// Placeholder.
#for _ in (1, 2) {}
#for _ in (a: 1, b: 2) {}
#for _ in "foo" {}
#for _ in bytes("😊") {}

--- issue-3275-destructuring ---
// Destructuring.
#for (a,b,c) in (("a", 1, bytes(())), ("b", 2, bytes(""))) {}
#for (a, ..) in (("a", 1, bytes(())), ("b", 2, bytes(""))) {}
#for (k, v)  in (a: 1, b: 2, c: 3) {}
#for (.., v) in (a: 1, b: 2, c: 3) {}

--- issue-3275-loop-over-content ---
// Error: 11-17 cannot loop over content
#for x in [1, 2] {}

--- issue-3275-loop-over-arguments ---
// Error: 11-25 cannot loop over arguments
#for _ in arguments("a") {}

--- issue-3275-loop-over-integer ---
// Error: 16-21 cannot loop over integer
#for (x, y) in 12306 {}

--- issue-3275-destructuring-loop-over-content ---
// Error: 16-22 cannot loop over content
#for (x, y) in [1, 2] {}

--- issue-3275-destructuring-loop-over-string ---
// Error: 6-12 cannot destructure values of string
#for (x, y) in "foo" {}

--- issue-3275-destructuring-loop-over-string-array ---
// Error: 6-12 cannot destructure string
#for (x, y) in ("foo", "bar") {}

--- issue-3275-destructuring-loop-over-bytes ---
// Error: 6-12 cannot destructure values of bytes
#for (x, y) in bytes("😊") {}

--- issue-3275-destructuring-loop-over-bytes-array ---
// Error: 6-12 cannot destructure bytes
#for (x, y) in (bytes((1,2)), bytes((1,2))) {}

--- issue-3275-destructuring-loop-over-int-array ---
// Error: 6-12 cannot destructure integer
#for (x, y) in (1, 2) {}

--- issue-3275-destructuring-loop-over-2d-array-1 ---
// Error: 10-11 not enough elements to destructure
// Hint: 10-11 the provided array has a length of 1, but the pattern expects 2 elements
#for (x, y) in ((1,), (2,)) {}

--- issue-3275-destructuring-loop-over-2d-array-2 ---
// Error: 6-12 too many elements to destructure
// Hint: 6-12 the provided array has a length of 3, but the pattern expects 2 elements
#for (x, y) in ((1,2,3), (4,5,6)) {}

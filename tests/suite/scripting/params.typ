--- param-underscore-missing-argument render ---
// Error: 17-20 missing argument: pattern parameter
#let f(a: 10) = a() + 1
#f(a: _ => 5)

--- params-sink-named render ---
// ... but this was.
#let f(..x) = {}
#f(arg: 1)

--- params-sink-unnamed render ---
// unnamed spread
#let f(.., a) = a
#test(f(1, 2, 3), 3)

// This wasn't allowed before the bug fix ...
#let f(..) = 2
#test(f(arg: 1), 2)

--- params-sink-bool-invalid render ---
// Error: 10-14 expected pattern, found boolean
#let f(..true) = none

--- params-sink-multiple-invalid render ---
// Error: 13-16 only one argument sink is allowed
#let f(..a, ..b) = none

--- params-sink-at-start render ---
// Spread at beginning.
#{
  let f(..a, b) = (a, b)
  test(repr(f(1)), "(arguments(), 1)")
  test(repr(f(1, 2, 3)), "(arguments(1, 2), 3)")
  test(repr(f(1, 2, 3, 4, 5)), "(arguments(1, 2, 3, 4), 5)")
}

--- params-sink-in-middle render ---
// Spread in the middle.
#{
  let f(a, ..b, c) = (a, b, c)
  test(repr(f(1, 2)), "(1, arguments(), 2)")
  test(repr(f(1, 2, 3, 4, 5)), "(1, arguments(2, 3, 4), 5)")
}

--- params-sink-unnamed-empty render ---
// Unnamed sink should just ignore any extra arguments.
#{
  let f(a, b: 5, ..) = (a, b)
  test(f(4), (4, 5))
  test(f(10, b: 11), (10, 11))
  test(f(13, 20, b: 12), (13, 12))
  test(f(15, b: 16, c: 13), (15, 16))
}

--- params-sink-missing-arguments render ---
#{
  let f(..a, b, c, d) = none

  // Error: 3-10 missing argument: d
  f(1, 2)
}

--- issue-1029-parameter-destructuring render ---
// Test that underscore works in parameter patterns.
#test((1, 2, 3).zip((1, 2, 3)).map(((_, x)) => x), (1, 2, 3))

--- issue-1351-parameter-dictionary render ---
// Error: 17-22 expected pattern, found string
#let foo((test: "bar")) = {}

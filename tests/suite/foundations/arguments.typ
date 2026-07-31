// Test arguments.

--- arguments-len eval ---
#test(arguments().len(), 0)
#test(arguments("hello").len(), 1)
#test(arguments(a: "world").len(), 1)
#test(arguments(a: "hey", 14).len(), 2)
#test(arguments(0, 1, a: 2, 3).len(), 4)

--- arguments-at eval ---
#let args = arguments(0, 1, a: 2, 3)
#test(args.at(0), 0)
#test(args.at(1), 1)
#test(args.at(2), 3)
#test(args.at("a"), 2)

--- arguments-field-access eval ---
#let args = arguments(0, 1, a: 2, 3, b: arguments(5, c: 4))
#test(args.a, 2)
#test(args.b, arguments(5, c: 4))
#test(args.b.c, 4)
#test(args.b.at(0), 5)
#test(args.b.at("c"), 4)
#test(args.at("b").c, 4)

--- arguments-at-call eval ---
// Test calling a function in an argument via `.at()`.
#let args = arguments(x => x + 1, func: x => x + 2)
#test(args.at(0)(0), 1)
#test(args.at("func")(0), 2)

--- arguments-at-index-missing eval ---
#let args = arguments(0, 1, a: 2, 3)
// Error: 2-12 no positional argument at index 4 and no default value was specified
#args.at(4)

--- arguments-at-name-missing eval ---
#let args = arguments(0, 1, a: 2, 3)
// Error: 2-14 no named argument "b" and no default value was specified
#args.at("b")

--- arguments-field-missing eval ---
#let args = arguments(0, 1, a: 2, 3)
// Error: 7-8 no named argument "b"
#args.b

--- arguments-field-invalid-syntax eval ---
#let args = arguments(0)
// Error: 11-11 expected comma
#test(args.0, 0)

--- arguments-field-assign eval ---
#{
  let args = arguments(a: 1)
  // Error: 3-7 cannot mutate fields on arguments
  args.a = 2
  test(args.a, 2)
}

--- arguments-at-assign-named eval ---
#{
  let args = arguments(0, a: 1)
  // Error: 3-15 cannot mutate a temporary value
  args.at("a") = 2
  test(args.a, 2)
}

--- arguments-at-assign-pos eval ---
#{
  let args = arguments(0, a: 1)
  // Error: 3-13 cannot mutate a temporary value
  args.at(0) = 2
  test(args.at(0), 2)
}

--- arguments-field-assign-missing eval ---
#{
  let args = arguments(a: 1)
  // Error: 3-7 cannot mutate fields on arguments
  args.b = 2
}

--- arguments-at-assign-missing-name eval ---
#{
  let args = arguments(a: 1)
  // Error: 3-15 cannot mutate a temporary value
  args.at("b") = 2
}

--- arguments-at-assign-missing-index eval ---
#{
  let args = arguments(0, a: 1)
  // Error: 3-13 cannot mutate a temporary value
  args.at(1) = 2
}

--- arguments-plus-sum-join eval ---
#let lhs = arguments(0, "1", key: "value", 3)
#let rhs = arguments(other-key: 4, key: "other value", 3)
#let result = arguments(0, "1", 3, other-key: 4, key: "other value", 3)
#test(lhs + rhs, result)
#test({lhs; rhs}, result)
#test((lhs, rhs).sum(), result)
#test((lhs, rhs).join(), result)

--- arguments-filter eval ---
// Test the `filter` method.
#test(arguments().filter(calc.even), arguments())
#test(arguments(1, a: 2, b: 3, 4).filter(calc.even), arguments(a: 2, 4))
#test(arguments(h: 7, e: 3, l: 2, o: 5, 1).filter(x => x < 5), arguments(e: 3, l: 2, 1))

--- arguments-filter-error eval ---
// Test that errors in the predicate are reported properly.
// Error: 29-34 cannot subtract integer from string
#arguments("a").filter(x => x - 2)

--- arguments-map eval ---
// Test the `map` method.
#test(arguments().map(x => x * 2), arguments())
#test(arguments(2, a: 3).map(x => x * 2), arguments(4, a: 6))

--- arguments-map-error eval ---
// Test that errors in the function are reported properly.
// Error: 26-31 cannot subtract integer from string
#arguments("a").map(x => x - 2)

--- arguments-method-typo eval ---
#let args = arguments(0)
// Error: 2-10 type arguments has no method `att`
#args.att(0)

// Test arguments.

--- arguments-len ---
#test(arguments().len(), 0)
#test(arguments("hello").len(), 1)
#test(arguments(a: "world").len(), 1)
#test(arguments(a: "hey", 14).len(), 2)
#test(arguments(0, 1, a: 2, 3).len(), 4)

--- arguments-at ---
#let args = arguments(0, 1, a: 2, 3)
#test(args.at(0), 0)
#test(args.at(1), 1)
#test(args.at(2), 3)
#test(args.at("a"), 2)

--- arguments-at-invalid-index ---
#let args = arguments(0, 1, a: 2, 3)
// Error: 2-12 arguments do not contain key 4 and no default value was specified
#args.at(4)

--- arguments-at-invalid-name ---
#let args = arguments(0, 1, a: 2, 3)
// Error: 2-14 arguments do not contain key "b" and no default value was specified
#args.at("b")

--- arguments-plus-sum-join ---
#let lhs = arguments(0, "1", key: "value", 3)
#let rhs = arguments(other-key: 4, key: "other value", 3)
#let result = arguments(0, "1", 3, other-key: 4, key: "other value", 3)
#test(lhs + rhs, result)
#test({lhs; rhs}, result)
#test((lhs, rhs).sum(), result)
#test((lhs, rhs).join(), result)

--- arguments-filter ---
// Test the `filter` method.
#test(arguments().filter(calc.even), arguments())
#test(arguments(1, a: 2, b: 3, 4).filter(calc.even), arguments(a: 2, 4))
#test(arguments(h: 7, e: 3, l: 2, o: 5, 1).filter(x => x < 5), arguments(e: 3, l: 2, 1))

--- arguments-filter-error ---
// Test that errors in the predicate are reported properly.
// Error: 29-34 cannot subtract integer from string
#arguments("a").filter(x => x - 2)

--- arguments-map ---
// Test the `map` method.
#test(arguments().map(x => x * 2), arguments())
#test(arguments(2, a: 3).map(x => x * 2), arguments(4, a: 6))

--- arguments-map-error ---
// Test that errors in the function are reported properly.
// Error: 26-31 cannot subtract integer from string
#arguments("a").map(x => x - 2)

// Test arguments.

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

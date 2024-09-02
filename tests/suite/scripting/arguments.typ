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

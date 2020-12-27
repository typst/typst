// Test error cases of the `font` function.

// Not one of the valid things for positional arguments.
[font false]

// Wrong types.
[font style: bold, weight: "thin", serif: 0]

// Non-existing argument.
[font something: "invalid"]

// compare-ref: false
// error: 4:7-4:12 unexpected argument
// error: 7:14-7:18 invalid font style
// error: 7:28-7:34 expected font weight, found string
// error: 7:43-7:44 expected family or list of families, found integer
// error: 10:7-10:27 unexpected argument

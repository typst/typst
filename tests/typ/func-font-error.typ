// Test error cases of the `font` function.

// Not one of the valid things for positional arguments.
[font: false]

// Wrong types.
[font: style=bold, weight="thin", emoji=0]

// Non-existing argument.
[font: something="invalid"]

// compare-ref: false
// error: 4:8-4:13 unexpected argument
// error: 7:14-7:18 invalid font style
// error: 7:27-7:33 expected font weight, found string
// error: 7:41-7:42 expected family or list of families, found integer
// error: 10:8-10:27 unexpected argument

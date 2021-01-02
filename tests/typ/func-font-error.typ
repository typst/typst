// Test error cases of the `font` function.

// Not one of the valid things for positional arguments.
[font false]

// Wrong types.
[font style: bold, weight: "thin", serif: 0]

// Weight out of range.
[font weight: 2700]

// Non-existing argument.
[font something: "invalid"]

// compare-ref: false
// error: 4:7-4:12 unexpected argument
// error: 7:14-7:18 expected font style, found font weight
// error: 7:28-7:34 expected font weight, found string
// error: 7:43-7:44 expected font family or array of font families, found integer
// warning: 10:15-10:19 must be between 100 and 900
// error: 13:7-13:27 unexpected argument

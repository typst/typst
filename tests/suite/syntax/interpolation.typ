// Syntax tests for string interpolation.

--- interpolation-optional-explicit-0-14 compat(0.14) eval ---
// On preferred version 0.14 string interpolation is _not_ parsed.
#let value = "Hello"
#test("#value World", "#value World")

--- interpolation-optional-explicit-0-15 compat(0.15) eval ---
// On preferred version 0.15 string interpolation is parsed.
#let value = "Hello"
#test("#value World", "Hello World")

--- interpolation-optional-current eval ---
// The default preferred version is the current one, on which interpolation is
// parsed.
#let value = "Hello"
#test("#value World", "Hello World")

--- interpolation-semicolon eval ---
// A semicolon exits the expression parsing early just like in markup.
#let value = "Hello"
#test("#value;World", "HelloWorld")

--- interpolation-chained-expression eval ---
// A chained expression is parsed fully, just like in markup.
#let value = "Hello"
#test("#value.slice(0, -1) World", "Hell World")

--- interpolation-error-span eval ---
// Span points to identifier.
// Error: 4-9 unknown variable: value
#"#value World"

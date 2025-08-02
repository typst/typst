// Test math symbol edge cases.

--- math-symbol-basic ---
#let sym = symbol("s", ("basic", "s"))
#test($sym.basic$, $s$)

--- math-symbol-underscore ---
#let sym = symbol("s", ("test_underscore", "s"))
// Error: 6-10 unknown symbol modifier
// Error: 11-21 unknown variable: underscore
// Hint: 11-21 if you meant to display multiple letters as is, try adding spaces between each letter: `u n d e r s c o r e`
// Hint: 11-21 or if you meant to display this as text, try placing it in quotes: `"underscore"`
$sym.test_underscore$

--- math-symbol-dash ---
#let sym = symbol("s", ("test-dash", "s"))
// Error: 6-10 unknown symbol modifier
$sym.test-dash$

--- math-symbol-double ---
#let sym = symbol("s", ("test.basic", "s"))
#test($sym.test.basic$, $s$)

--- math-symbol-double-underscore ---
#let sym = symbol("s", ("one.test_underscore", "s"))
// Error: 10-14 unknown symbol modifier
// Error: 15-25 unknown variable: underscore
// Hint: 15-25 if you meant to display multiple letters as is, try adding spaces between each letter: `u n d e r s c o r e`
// Hint: 15-25 or if you meant to display this as text, try placing it in quotes: `"underscore"`
$sym.one.test_underscore$

--- math-symbol-double-dash ---
#let sym = symbol("s", ("one.test-dash", "s"))
// Error: 10-14 unknown symbol modifier
$sym.one.test-dash$

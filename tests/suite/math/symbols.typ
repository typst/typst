// Test math symbol edge cases.

--- math-symbol-basic paged ---
#let sym = symbol("s", ("basic", "s"))
#test($sym.basic$, $s$)

--- math-symbol-underscore paged ---
#let sym = symbol("s", ("test_underscore", "s"))
// Error: 6-10 unknown symbol modifier
$sym.test_underscore$

--- math-symbol-dash paged ---
#let sym = symbol("s", ("test-dash", "s"))
// Error: 6-10 unknown symbol modifier
$sym.test-dash$

--- math-symbol-double paged ---
#let sym = symbol("s", ("test.basic", "s"))
#test($sym.test.basic$, $s$)

--- math-symbol-double-underscore paged ---
#let sym = symbol("s", ("one.test_underscore", "s"))
// Error: 10-14 unknown symbol modifier
$sym.one.test_underscore$

--- math-symbol-double-dash paged ---
#let sym = symbol("s", ("one.test-dash", "s"))
// Error: 10-14 unknown symbol modifier
$sym.one.test-dash$

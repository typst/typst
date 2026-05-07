// Tests for panics in the parser caused by syntax.

--- issue-4571-panic-when-compiling-invalid-file eval ---
// Test that trying to parse the following does not result in a panic.

// Error: 9-10 unclosed delimiter
// Error: 22 expected pattern
// Error: 23-24 unexpected star
#import (hntle-clues: *
// Error: 1-2 the character `#` is not valid in code
// Hint: 1-2 you are already in code mode
// Hint: 1-2 try removing the `#`
// Error: 2-8 expected pattern, found keyword `import`
// Hint: 2-8 keyword `import` is not allowed as an identifier; try `import_` instead
// Error: 9-20 expected identifier, found string
#import "/util.typ": qrlink
// Error: 1-2 the character `#` is not valid in code
// Hint: 1-2 you are already in code mode
// Hint: 1-2 try removing the `#`
// Error: 2-5 expected pattern, found keyword `let`
// Hint: 2-5 keyword `let` is not allowed as an identifier; try `let_` instead
#let auton(
// Error: 3-4 unexpected equals sign
// Error: 5-6 unclosed delimiter
// Error: 6 expected equals sign
) = {

--- issue-4571-panic-when-compiling-invalid-file ---
// Test that trying to parse the following does not result in a panic.

// Error: 1:9-10 unclosed delimiter
// Error: 1:22 expected pattern
// Error: 1:23-24 unexpected star
// Error: 2:1-2:2 the character `#` is not valid in code
// Error: 2:2-2:8 expected pattern, found keyword `import`
// Hint: 2:2-2:8 keyword `import` is not allowed as an identifier; try `import_` instead
// Error: 2:9-2:20 expected identifier, found string
// Error: 3:1-3:2 the character `#` is not valid in code
// Error: 3:2-3:5 expected pattern, found keyword `let`
// Hint: 3:2-3:5 keyword `let` is not allowed as an identifier; try `let_` instead
// Error: 4:3-4:4 unexpected equals sign
// Error: 4:5-4:6 unclosed delimiter
// Error: 4:6 expected equals sign
#import (hntle-clues: *
#import "/util.typ": qrlink
#let auton(
) = {


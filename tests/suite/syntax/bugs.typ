--- syntax-bugs-extra-hash paged ---
#{
  // Error: 3-4 the character `#` is not valid in code
  // Hint: 3-4 you are already in code mode
  // Hint: 3-4 try removing the `#`
  #
}

--- syntax-bugs-bad-boolean-ops paged ---
// Test writing invalid boolean operators:
#{
  // Error: 3-5 `&&` is not valid in code
  // Hint: 3-5 in Typst, `and` is used for logical AND
  &&
  // Error: 3-4 the character `&` is not valid in code
  // Hint: 3-4 try removing the `&`
  &
  // Error: 3-5 `||` is not valid in code
  // Hint: 3-5 in Typst, `or` is used for logical OR
  ||
  // Error: 3-4 the character `|` is not valid in code
  // Hint: 3-4 try removing the `|`
  |
  // Error: 3-4 the character `!` is not valid in code
  // Hint: 3-4 in Typst, `not` is used for negation
  // Hint: 3-4 or did you mean to write `!=` for not-equal?
  !
  // Error: 3-4 the character `!` is not valid in code
  // Hint: 3-4 in Typst, `not` is used for negation
  // Hint: 3-4 or did you mean to write `!=` for not-equal?
  // Error: 5-6 unexpected equals sign
  ! =
  // Error: 3-4 the character `~` is not valid in code
  // Hint: 3-4 try removing the `~`
  ~
  // Error: 3-5 `~=` is not valid in code
  // Hint: 3-5 in Typst, `!=` is used for not-equal
  ~=
}

--- syntax-bugs-bad-boolean-after-hash paged ---
// We give different hints than above directly after a hash
// Error: 2-3 the character `!` is not valid in code
// Hint: 2-3 try removing the `!`
#!
// Error: 2-3 the character `&` is not valid in code
// Hint: 2-3 try removing the `&`
#&&

--- issue-4571-panic-when-compiling-invalid-file paged ---
// Test that trying to parse the following does not result in a panic.

// Error: 1:9-10 unclosed delimiter
// Error: 1:22 expected pattern
// Error: 1:23-24 unexpected star
// Error: 2:1-2:2 the character `#` is not valid in code
// Hint: 2:1-2:2 you are already in code mode
// Hint: 2:1-2:2 try removing the `#`
// Error: 2:2-2:8 expected pattern, found keyword `import`
// Hint: 2:2-2:8 keyword `import` is not allowed as an identifier; try `import_` instead
// Error: 2:9-2:20 expected identifier, found string
// Error: 3:1-3:2 the character `#` is not valid in code
// Hint: 3:1-3:2 you are already in code mode
// Hint: 3:1-3:2 try removing the `#`
// Error: 3:2-3:5 expected pattern, found keyword `let`
// Hint: 3:2-3:5 keyword `let` is not allowed as an identifier; try `let_` instead
// Error: 4:3-4:4 unexpected equals sign
// Error: 4:5-4:6 unclosed delimiter
// Error: 4:6 expected equals sign
#import (hntle-clues: *
#import "/util.typ": qrlink
#let auton(
) = {

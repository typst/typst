// Syntax tests for code mode and embedded code expressions.

--- embedded-code-incomplete eval ---
// Error: 2-2 expected expression
#

--- embedded-code-incomplete-followed-by-text eval ---
// Error: 2-2 expected expression
#  hello

--- embedded-code-extra-hash eval ---
// The span on the hints isn't great, but it's hard to fix.

// Error: 2-3 the character `#` is not valid in code
// Hint: 2-3 the preceding hash is causing this to parse in code mode
// Hint: 2-3 try escaping the preceding hash: `\#`
##

// Error: 2-3 the character `#` is not valid in code
// Hint: 2-3 the preceding hash is causing this to parse in code mode
// Hint: 2-3 try escaping the preceding hash: `\#`
// Error: 4-5 the character `#` is not valid in code
// Hint: 4-5 the preceding hash is causing this to parse in code mode
// Hint: 4-5 try escaping the preceding hash: `\#`
####

--- embedded-code-invalid-character eval ---
// Error: 2-3 the character `&` is not valid in code
// Hint: 2-3 the preceding hash is causing this to parse in code mode
// Hint: 2-3 try escaping the preceding hash: `\#`
#&&
// Error: 2-3 the character `!` is not valid in code
// Hint: 2-3 the preceding hash is causing this to parse in code mode
// Hint: 2-3 try escaping the preceding hash: `\#`
#!

--- code-syntax-extra-hash eval ---
// Test unneeded hashtags in code mode.
// Similar tests for markup exist in "syntax/embedded.typ"
#{
  // Error: 3-4 the character `#` is not valid in code
  // Hint: 3-4 you are already in code mode
  // Hint: 3-4 try removing the `#`
  #
  // Error: 3-4 the character `#` is not valid in code
  // Hint: 3-4 you are already in code mode
  // Hint: 3-4 try removing the `#`
  // Error: 4-5 the character `#` is not valid in code
  // Hint: 4-5 the preceding hash is causing this to parse in code mode
  // Hint: 4-5 try escaping the preceding hash: `\#`
  // Error: 5-6 the character `#` is not valid in code
  // Hint: 5-6 the preceding hash is causing this to parse in code mode
  // Hint: 5-6 try escaping the preceding hash: `\#`
  ###
}

--- code-syntax-bad-boolean-ops eval ---
// Test writing invalid boolean operators:
#{
  // Error: 3-5 `&&` is not valid in code
  // Hint: 3-5 in Typst, `and` is used for logical AND
  &&
  // Error: 3-4 the character `&` is not valid in code
  &
  // Error: 3-5 `||` is not valid in code
  // Hint: 3-5 in Typst, `or` is used for logical OR
  ||
  // Error: 3-4 the character `|` is not valid in code
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
  ~
  // Error: 3-5 `~=` is not valid in code
  // Hint: 3-5 in Typst, `!=` is used for not-equal
  ~=
}

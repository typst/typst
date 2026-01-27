// Test shebang support.

--- shebang paged ---
#!typst compile

// Error: 2-3 the character `!` is not valid in code
// Hint: 2-3 in Typst, `not` is used for negation
// Hint: 2-3 or did you mean to write `!=` for not-equal?
#!not-a-shebang

// Test shebang support.

--- shebang paged ---
#!typst compile

// Error: 2-3 the character `!` is not valid in code
// Hint: 2-3 try removing the `!`
#!not-a-shebang

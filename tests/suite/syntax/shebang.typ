// Test shebang support.

--- shebang paged ---
#!typst compile

// Error: 2-3 the character `!` is not valid in code
// Hint: 2-3 the preceding hash is causing this to parse in code mode
// Hint: 2-3 try escaping the preceding hash: `\#`
#!not-a-shebang

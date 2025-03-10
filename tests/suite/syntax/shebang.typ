// Test shebang support.

--- shebang ---
#!typst compile

// Error: 2-3 the character `!` is not valid in code
#!not-a-shebang

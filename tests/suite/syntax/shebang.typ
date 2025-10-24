// Test shebang support.

--- shebang render ---
#!typst compile

// Error: 2-3 the character `!` is not valid in code
#!not-a-shebang

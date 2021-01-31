// Test line and block comments.

---
// Line comment acts as spacing.
A// you
B

// Block comment does not act as spacing.
C/*
 /* */
*/D

// Works in code.
#[f /*1*/ a: "b" //
, 1]

---
// End should not appear without start.
// Error: 1:7-1:9 unexpected end of block comment
/* */ */

// Unterminated is okay.
/*

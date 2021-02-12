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
#[test type /*1*/ (1) //
, "integer"]

---
// End should not appear without start.
// Error: 1:7-1:9 unexpected end of block comment
/* */ */

// Unterminated is okay.
/*

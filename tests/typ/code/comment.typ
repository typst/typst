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
#test(type(/*1*/ 1) //
, "integer")

---
// Line comments have a special case for URLs.
https://example.com \
https:/* block comments don't ... */

---
// End should not appear without start.
// Error: 1:7-1:9 unexpected end of block comment
/* */ */

// Unterminated is okay.
/*

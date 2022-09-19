// Test line and block comments.

---
// Line comment acts as spacing.
A// you
B

// Block comment does not act as spacing, nested block comments.
C/*
 /* */
*/D

// Works in code.
#test(type(/*1*/ 1) //
, "integer")

// End of block comment in line comment.
// Hello */

// Nested line comment.
/*//*/
Still comment.
*/

E

---
// Line comments have a special case for URLs.
https://example.com \
https:/* block comments don't ... */

---
// End should not appear without start.
// Error: 7-9 unexpected end of block comment
/* */ */

// Unterminated is okay.
/*

// Test interaction with words, spacing and expressions.

A// you
B

C/*
 /* */
*/D

[dump /*1*/ a: "b" //
, 1]

---
// Test error.
//
// ref: false
// error: 3:7-3:9 unexpected end of block comment

// No start of block comment.
/* */ */

// Unterminated block comment is okay.
/*

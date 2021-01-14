// Test whether line comment acts as spacing.
A// you
B

// Test whether block comment acts as spacing.
C/*
 /* */
*/D

// Test in expressions.
[dump /*1*/ a: "b" //
, 1]

// Error: 1:7-1:9 unexpected end of block comment
/* */ */

// Unterminated block comment is okay.
/*

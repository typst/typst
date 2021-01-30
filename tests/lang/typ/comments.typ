// Line comment acts as spacing.
A// you
B

// Block comment does not act as spacing.
C/*
 /* */
*/D

// Works in headers.
#[f /*1*/ a: "b" //
, 1]

// End should not appear without start.
// Error: 7-9 unexpected end of block comment
/* */ */

// Unterminated is okay.
/*

// Test line and block comments.

--- comments ---
// Line comment acts as spacing.
A// you
B

// Block comment does not act as spacing, nested block comments.
C/*
 /* */
*/D

// Works in code.
#test(type(/*1*/ 1) //
, int)

// End of block comment in line comment.
// Hello */

// Nested "//" doesn't count as line comment.
/* // */
E

/*//*/
This is a comment.
*/*/

--- comment-end-of-line ---
// Test comments at the end of a line
First part//
Second part

// Test comments at the end of a line with pre-spacing
First part          //
Second part

--- comment-block-unclosed ---
// End should not appear without start.
// Error: 7-9 unexpected end of block comment
/* */ */

// Unterminated is okay.
/*

// Test whitespace handling.

---
// Test that a run consisting only of whitespace isn't trimmed.
A#font("PT Sans")[ ]B

---
// Test font change after space.
Left #font("PT Sans")[Right].

---
// Test that space at start of line is not trimmed.
A{"\n"} B

---
// Test that trailing space does not force a line break.
LLLLLLLLLLLLLL R _L_

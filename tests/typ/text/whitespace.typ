// Test whitespace handling.

---
// Spacing around let.

// Error: 6 expected identifier
A#let;B \
A#let x = 1;B  #test(x, 1) \
A #let x = 2;B #test(x, 2) \
A#let x = 3; B #test(x, 3)

---
// Spacing around if-else.

A#if true [B]C \
A#if true [B] C \
A #if true{"B"}C \
A #if true{"B"} C \
A#if false [] #else [B]C \
A#if true [B] #else [] C

---
// Spacing around while loop.

#let c = true; A#while c [{c = false}B]C \
#let c = true; A#while c [{c = false}B] C \
#let c = true; A #while c { c = false; "B" }C \
#let c = true; A #while c { c = false; "B" } C

---
// Spacing around for loop.

A#for _ in (none,) [B]C  \
A#for _ in (none,) [B] C \
A #for _ in (none,) {"B"}C

---
// Test that a run consisting only of whitespace isn't trimmed.
A#font(family: "PT Sans")[ ]B

---
// Test font change after space.
Left #font(family: "PT Sans")[Right].

---
// Test that space at start of line is not trimmed.
A{"\n"} B

---
// Test that trailing space does not force a line break.
LLLLLLLLLLLLLL R _L_

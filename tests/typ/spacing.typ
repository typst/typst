// Test spacing around control flow structures.

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

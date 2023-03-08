// Test whitespace handling.

---
// Spacing around code constructs.
A#let x = 1;B  #test(x, 1) \
C #let x = 2;D #test(x, 2) \
E#if true [F]G \
H #if true{"I"} J \
K #if true [L] else []M \
#let c = true; N#while c [#(c = false)O] P \
#let c = true; Q #while c { c = false; "R" } S \
T#for _ in (none,) {"U"}V
#let foo = "A" ; \
#foo;B \
#foo; B \
#foo ;B

---
// Test spacing with comments.
A/**/B/**/C \
A /**/ B/**/C \
A /**/B/**/ C

---
// Test that a run consisting only of whitespace isn't trimmed.
A#text(font: "IBM Plex Serif")[ ]B

---
// Test font change after space.
Left #text(font: "IBM Plex Serif")[Right].

---
// Test that linebreak consumed surrounding spaces.
#align(center)[A \ B \ C]

---
// Test that space at start of non-backslash-linebreak line isn't trimmed.
A#"\n" B

---
// Test that trailing space does not force a line break.
LLLLLLLLLLLLLLLLLL R _L_

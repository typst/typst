// Test whitespace handling.

--- space-collapsing paged ---
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

--- space-collapsing-comments paged ---
// Test spacing with comments.
A/**/B/**/C \
A /**/ B/**/C \
A /**/B/**/ C

--- space-collapsing-with-h paged ---
// Test spacing collapsing before spacing.
#set align(right)
A #h(0pt) B #h(0pt) \
A B \
A #h(-1fr) B

--- text-font-just-a-space paged ---
// Test that a run consisting only of whitespace isn't trimmed.
A#text(font: "IBM Plex Serif")[ ]B

--- text-font-change-after-space paged ---
// Test font change after space.
Left #text(font: "IBM Plex Serif")[Right].

--- space-collapsing-linebreaks paged ---
// Test that linebreak consumed surrounding spaces.
#align(center)[A \ B \ C]

--- space-collapsing-stringy-linebreak paged ---
// Test that space at start of non-backslash-linebreak line isn't trimmed.
A#"\n" B

--- space-trailing-linebreak paged ---
// Test that trailing space does not force a line break.
LLLLLLLLLLLLLLLLLL R _L_

--- space-ideographic-kept paged ---
// Test that ideographic spaces are preserved.
#set text(lang: "ja", font: "Noto Serif CJK JP")

だろうか？　何のために！　私は、

--- space-thin-kept paged ---
// Test that thin spaces are preserved.
| | U+0020 regular space \
| | U+2009 thin space

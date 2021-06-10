// Test configuring paragraph properties.

---
// FIXME: Word spacing doesn't work due to new shaping process.
#par(spacing: 10pt, leading: 25%, word-spacing: 1pt)

But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun.

---
// Test that it finishes an existing paragraph.
Hello #par(word-spacing: 0pt) t h e r e !

// Test text alignment.

---
// Test that alignment depends on the paragraph's full width.
#box[
  Hello World \
  #align(right)[World]
]

---
// Test that a line with multiple alignments respects the paragraph's full
// width.
#box[
  Hello #align(center)[World] \
  Hello from the World
]

---
// Test that `start` alignment after `end` alignment doesn't do anything until
// the next line break ...
L #align(right)[R] R

// ... but make sure it resets to left after the line break.
L #align(right)[R] \ L

---
// FIXME: There should be a line break opportunity on alignment change.
LLLLLLLLLLLLLLLLL#align(center)[CCCC]

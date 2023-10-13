// Test shorthands for unicode codepoints.

---
The non-breaking~space does work.

---
// Make sure non-breaking and normal space always
// have the same width. Even if the font decided
// differently.
#set text(font: "New Computer Modern")
a b \
a~b

---
- En dash: --
- Em dash: ---

---
#set text(font: "Roboto")
A... vs #"A..."

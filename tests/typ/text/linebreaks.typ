// Test line breaking special cases.

---
// Test overlong word that is not directly after a hard break.
This is a spaceexceedinglylongishy.

---
// Test two overlong words in a row.
Supercalifragilisticexpialidocious Expialigoricmetrioxidation.

---
// Test that there are no unwanted line break opportunities on run change.
This is partly emph_as_ized.

---
Hard \ break.

---
// Test hard break directly after normal break.
Hard break directly after \ normal break.

---
// Test consecutive breaks.
Two consecutive \ \ breaks and three \ \ \ more.

---
// Test trailing newline.
Trailing break \

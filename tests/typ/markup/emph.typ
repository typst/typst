// Test emphasis toggle.

---
// Basic.
_Emphasized!_

// Inside of words.
Partly em_phas_ized.

// Scoped to body.
#rect[_Scoped] to body.

---
#let emph = strong
_Strong_

#let emph() = "Hi"
_, _!

#let emph = "hi"

// Error: 1-2 expected function, found string
_

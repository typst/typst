// Test strong toggle.

---
// Basic.
*Strong!*

// Inside of words.
Partly str*ength*ened.

// Scoped to body.
#rect[*Scoped] to body.

---
#let strong = emph
*Emph*

#let strong() = "Bye"
*, *!

#let strong = 123
// Error: 1-2 expected function, found integer
*

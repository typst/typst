// Test the `lang` function.

---
Left to right.

#lang("ar")
Right to left.

#lang(dir: ltr)
Back again.

---
// Ref: false

// Error: 12-15 must be horizontal
#lang(dir: ttb)

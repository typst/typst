// Test the `h` and `v` functions.

---
// Ends paragraphs.
Tightly #v(0pt) packed

// Eating up soft spacing.
Inv #h(0pt) isible

// Multiple spacings in a row.
Add #h(10pt) #h(10pt) up

// Relative to area.
#let x = 25% - 4pt
| #h(x) | #h(x) | #h(x) | #h(x) |

---
// Missing spacing.
// Error: 11-13 missing argument: spacing
Totally #h() ignored

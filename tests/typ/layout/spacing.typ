// Test the `h` and `v` functions.

---
// Linebreak and v(0pt) are equivalent.
#box[A \ B] #box[A #v(0pt) B]

// Eating up soft spacing.
Inv#h(0pt)isible

// Multiple spacings in a row.
Add #h(10pt) #h(10pt) up

// Relative to area.
#let x = 25% - 4pt
|#h(x)|#h(x)|#h(x)|#h(x)|

// Fractional.
| #h(1fr) | #h(2fr) | #h(1fr) |

---
// Test spacing collapsing before spacing.
#set par(align: right)
A #h(0pt) B #h(0pt) \
A B

---
// Missing spacing.
// Error: 11-13 missing argument: spacing
Totally #h() ignored

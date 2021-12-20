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

// Fractional.
| #h(1fr) | #h(2fr) | #h(1fr) |

---
// Test spacing collapsing with parbreaks.
#v(0pt)
A
#v(0pt)
B
#v(0pt)

C #parbreak() D

---
// Test that spacing can carry paragraph and page style properties.

A[#set par(align: right);#h(1cm)]B
[#set page(height: 20pt);#v(1cm)]
B

---
// Missing spacing.
// Error: 11-13 missing argument: spacing
Totally #h() ignored

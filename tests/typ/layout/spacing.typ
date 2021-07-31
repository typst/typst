// Test the `h` and `v` functions.

---
// Ends paragraphs.
Tightly #v(0pt) packed

// Eating up soft spacing.
Inv #h(0pt) isible

// Multiple spacings in a row.
Add #h(10pt) #h(10pt) up

// Relative to font size.
Relative #h(100%) spacing

---
// Missing spacing.
// Error: 11-13 missing argument: spacing
Totally #h() ignored

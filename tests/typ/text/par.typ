// Test configuring paragraph properties.

---
// Test ragged-left.
#set par(align: right)
To the right! Where the sunlight peeks behind the mountain.

---
// Test that explicit paragraph break respects active styles.
#set par(spacing: 0pt)
[#set par(spacing: 100pt);First]

[#set par(spacing: 100pt);Second]
#set par(spacing: 13.5pt)

Third

---
// Test that paragraph spacing uses correct set rule.
Hello

#set par(spacing: 100pt)
World
#set par(spacing: 0pt, leading: 0pt)

You

---
// Test that paragraphs break due to incompatibility has correct spacing.
A #set par(spacing: 0pt, leading: 0pt); B #parbreak() C

---
// Test weird metrics.
#set par(spacing: 1em, leading: 0pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

---
// Error: 17-20 must be horizontal
#set par(align: top)

---
// Error: 17-33 expected alignment, found 2d alignment
#set par(align: horizon + center)

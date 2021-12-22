// Test configuring paragraph properties.

---
// Test ragged-left.
#set par(align: right)
To the right! Where the sunlight peeks behind the mountain.

---
// Test that explicit paragraph break respects active styles.
#set par(spacing: 7pt)
[#set par(spacing: 100pt);First]

[#set par(spacing: 100pt);Second]
#set par(spacing: 20pt)

Third

---
// Test that paragraph spacing uses correct set rule.
Hello

#set par(spacing: 100pt)
World
#set par(spacing: 0pt)

You

---
// Test that paragraph break due to incompatibility respects
// spacing defined by the two adjacent paragraphs.
#let a = [#set par(spacing: 40pt);Hello]
#let b = [#set par(spacing: 60pt);World]
{a}{b}

---
// Test weird metrics.
#set par(spacing: 100%, leading: 0pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

---
// Error: 17-20 must be horizontal
#set par(align: top)

---
// Error: 17-33 expected alignment, found 2d alignment
#set par(align: horizon + center)

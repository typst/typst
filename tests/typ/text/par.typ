// Test configuring paragraph properties.

---
// Test ragged-left.
#par(align: right)
To the right! Where the sunlight peeks behind the mountain.

---
// Test that explicit paragraph break respects active styles.
#par(spacing: 7pt)
[#par(spacing: 100pt) First]

[#par(spacing: 100pt) Second]
#par(spacing: 20pt)

Third

---
// Test that paragraph break due to incompatibility respects
// spacing defined by the two adjacent paragraphs.
#let a = [#par(spacing: 40pt) Hello]
#let b = [#par(spacing: 60pt) World]
{a}{b}

---
// Test weird metrics.
#par(spacing: 100%, leading: 0pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

---
// Error: 13-16 must be horizontal
#par(align: top)

---
// Error: 13-29 expected alignment, found 2d alignment
#par(align: horizon + center)

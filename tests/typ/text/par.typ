// Test configuring paragraph properties.

---
// Test ragged-left.
#par(align: right)
To the right! Where the sunlight peeks behind the mountain.

---
// Test weird metrics.
#par(spacing: 100%, leading: 0pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

---
// Error: 13-16 must be horizontal
#par(align: top)

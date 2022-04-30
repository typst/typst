// Test configuring paragraph properties.

---
// Test ragged-left.
#set par(align: right)
To the right! Where the sunlight peeks behind the mountain.

---
// Test changing leading and spacing.
#set par(spacing: 1em, leading: 2pt)
But, soft! what light through yonder window breaks?

It is the east, and Juliet is the sun.

---
// Test that largest paragraph spacing wins.
#set par(spacing: 2.5pt)
[#set par(spacing: 15pt);First]
[#set par(spacing: 7.5pt);Second]
Third

Fourth

---
// Test that paragraph spacing loses against block spacing.
#set par(spacing: 100pt)
#set table(around: 5pt)
Hello
#table(columns: 4, secondary: silver)[A][B][C][D]

---
// While we're at it, test the larger block spacing wins.
#set raw(around: 15pt)
#set math(around: 7.5pt)
#set list(around: 2.5pt)
#set par(spacing: 0pt)

```rust
fn main() {}
```

$[ x + y = z ]$

- List

Paragraph

---
// Error: 17-20 must be horizontal
#set par(align: top)

---
// Error: 17-33 expected alignment, found 2d alignment
#set par(align: horizon + center)

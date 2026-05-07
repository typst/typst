// Test divider element.

--- divider-basic paged ---
// Basic divider.
#set page(width: 200pt)
Before
#divider()
After

--- divider-multiple paged ---
// Test multiple dividers.
#set page(width: 200pt)
Section 1
#divider()
Section 2
#divider()
Section 3

--- divider-in-container paged ---
// Test divider in a container.
#set page(width: 200pt)
#box(width: 150pt, stroke: 1pt, inset: 10pt)[
  Content before
  #divider()
  Content after
]

--- divider-show-set-line paged ---
// Test customizing line via set rule.
#set page(width: 200pt)
#show divider: set line(stroke: 2pt + red)
Before
#divider()
After

--- divider-show-centered paged ---
// Test centered, shorter divider.
#set page(width: 200pt)
#show divider: block(
  width: 100%,
  spacing: 1em,
  align(center, line(length: 50%)),
)
Before
#divider()
After

--- divider-show-decorative paged ---
// Test replacing with custom content (asterism).
#set page(width: 200pt)
#show divider: set align(center)
#show divider: block[∗ ∗ ∗]
Chapter 1
#divider()
Chapter 2

--- divider-html html ---
// Test HTML output (should map to <hr>).
Introduction
#divider()
Body text
